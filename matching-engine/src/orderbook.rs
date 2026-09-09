use crate::api_messages::{
    CancelIDNotFoundError, CancelOccurredMessage, CancelRequest, JsonPayload, NewRestingOrderMessage,
    OrderFillMessage, OrderRequest, OutgoingMessage, TradeOccurredMessage,
};
use crate::config::{self, GlobalAccountState};
use crate::connection_server;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::SystemTime;
use actix::prelude::*;
use actix_web::web;
use std::cmp;

pub type OrderID = usize;
pub type Price = usize;
pub type TraderId = config::TraderId;
use serde::{Deserialize, Serialize, Serializer};
extern crate env_logger;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum OrderType {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Copy, Deserialize)]
pub struct Order {
    pub order_id: OrderID,
    pub trader_id: TraderId,
    pub symbol: config::TickerSymbol,
    pub amount: usize,
    pub price: Price,
    pub order_type: OrderType,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Fill {
    pub sell_trader_id: TraderId,
    pub buy_trader_id: TraderId,
    pub amount: usize,
    pub price: Price,
    pub symbol: config::TickerSymbol,
    pub trade_time: u8,
    pub resting_side: OrderType,
}

impl Message for Fill {
    type Result = ();
}

// ---------------------------------------------------------------------------
// Intrusive order-node arena
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct OrderNode {
    order: Order,
    next: Option<usize>,
    prev: Option<usize>,
}

/// A price level backed by an intrusive doubly-linked list of order nodes.
/// The queue ordering is FIFO (tail insert, head remove), preserving time priority.
#[derive(Debug, Default, Clone)]
struct Level {
    head: Option<usize>,
    tail: Option<usize>,
    /// Cached total quantity at this level, updated incrementally.
    total_amount: usize,
    len: usize,
}

impl Level {
    fn append(&mut self, node_idx: usize, nodes: &mut [OrderNode]) {
        self.total_amount += nodes[node_idx].order.amount;
        self.len += 1;
        nodes[node_idx].prev = self.tail;
        nodes[node_idx].next = None;
        if let Some(tail) = self.tail {
            nodes[tail].next = Some(node_idx);
        } else {
            self.head = Some(node_idx);
        }
        self.tail = Some(node_idx);
    }

    /// Remove a node from this level. Caller must ensure the node belongs to this level.
    fn remove(&mut self, node_idx: usize, nodes: &mut [OrderNode]) -> Order {
        let order = nodes[node_idx].order;
        self.total_amount -= order.amount;
        self.len -= 1;

        let prev = nodes[node_idx].prev;
        let next = nodes[node_idx].next;
        if let Some(p) = prev {
            nodes[p].next = next;
        } else {
            self.head = next;
        }
        if let Some(n) = next {
            nodes[n].prev = prev;
        } else {
            self.tail = prev;
        }
        nodes[node_idx].prev = None;
        nodes[node_idx].next = None;
        order
    }

    fn pop_front(&mut self, nodes: &mut [OrderNode]) -> Option<Order> {
        let head = self.head?;
        Some(self.remove(head, nodes))
    }
}

#[derive(Debug, Clone, Copy)]
struct OrderLocator {
    price: Price,
    side: OrderType,
    node_index: usize,
}

/// Dense order lookup by order ID. Order IDs are monotonically increasing and
/// dense enough for a vector to be faster than a hash map.
#[derive(Debug, Default, Clone)]
struct OrderIndex {
    locators: Vec<Option<OrderLocator>>,
}

impl OrderIndex {
    fn insert(&mut self, order_id: OrderID, locator: OrderLocator) {
        if order_id >= self.locators.len() {
            self.locators.resize(order_id + 1, None);
        }
        self.locators[order_id] = Some(locator);
    }

    fn remove(&mut self, order_id: OrderID) -> Option<OrderLocator> {
        let existing = self.locators.get_mut(order_id)?;
        existing.take()
    }
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct OrderBook {
    pub symbol: config::TickerSymbol,
    buy_side: BTreeMap<Price, Level>,
    sell_side: BTreeMap<Price, Level>,
    /// Arena of order nodes. Freed indices are recycled via `free_nodes`.
    nodes: Vec<OrderNode>,
    free_nodes: Vec<usize>,
    order_index: OrderIndex,
    pub price_history: Vec<(u64, u16, u16)>,
    /// Cached best bid (highest buy price). Invalidated lazily when the level empties.
    best_bid: Option<Price>,
    /// Cached best ask (lowest sell price). Invalidated lazily when the level empties.
    best_ask: Option<Price>,
}

impl Serialize for OrderBook {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("OrderBook", 4)?;
        s.serialize_field("symbol", &self.symbol)?;

        let buy_volumes: BTreeMap<Price, usize> = self
            .buy_side
            .iter()
            .map(|(&p, level)| (p, level.total_amount))
            .collect();
        s.serialize_field("buy_side", &buy_volumes)?;

        let sell_volumes: BTreeMap<Price, usize> = self
            .sell_side
            .iter()
            .map(|(&p, level)| (p, level.total_amount))
            .collect();
        s.serialize_field("sell_side", &sell_volumes)?;
        s.serialize_field("price_history", &self.price_history)?;
        s.end()
    }
}

impl OrderBook {
    fn allocate_node(&mut self, order: Order) -> usize {
        if let Some(idx) = self.free_nodes.pop() {
            self.nodes[idx] = OrderNode {
                order,
                next: None,
                prev: None,
            };
            idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(OrderNode {
                order,
                next: None,
                prev: None,
            });
            idx
        }
    }

    fn free_node(&mut self, idx: usize) {
        // Defensive: clear links to avoid accidental reuse of stale indices.
        self.nodes[idx].next = None;
        self.nodes[idx].prev = None;
        self.free_nodes.push(idx);
    }

    fn update_best_bid_after_remove(&mut self, price: Price) {
        if self.best_bid == Some(price) {
            self.best_bid = self.buy_side.keys().next_back().copied();
        }
    }

    fn update_best_ask_after_remove(&mut self, price: Price) {
        if self.best_ask == Some(price) {
            self.best_ask = self.sell_side.keys().next().copied();
        }
    }

    fn add_order_to_book(&mut self, new_order_request: OrderRequest, order_id: OrderID) -> Order {
        let new_order = Order {
            order_id,
            trader_id: new_order_request.trader_id,
            symbol: new_order_request.symbol,
            amount: new_order_request.amount,
            price: new_order_request.price,
            order_type: new_order_request.order_type,
        };
        let node_idx = self.allocate_node(new_order);

        {
            let level = match new_order.order_type {
                OrderType::Buy => self.buy_side.entry(new_order.price).or_default(),
                OrderType::Sell => self.sell_side.entry(new_order.price).or_default(),
            };
            level.append(node_idx, &mut self.nodes);
        }

        self.order_index.insert(
            order_id,
            OrderLocator {
                price: new_order.price,
                side: new_order.order_type,
                node_index: node_idx,
            },
        );

        // Update cached best price.
        match new_order.order_type {
            OrderType::Buy => {
                self.best_bid = Some(self.best_bid.map_or(new_order.price, |p| p.max(new_order.price)));
            }
            OrderType::Sell => {
                self.best_ask = Some(self.best_ask.map_or(new_order.price, |p| p.min(new_order.price)));
            }
        }

        new_order
    }

    pub fn handle_incoming_cancel_request(
        &mut self,
        cancel_request: CancelRequest,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        accounts_data: &GlobalAccountState,
    ) -> Result<Order, Box<dyn std::error::Error>> {
        let locator = self
            .order_index
            .remove(cancel_request.order_id)
            .ok_or_else(|| Box::new(CancelIDNotFoundError))?;

        if locator.side != cancel_request.side || locator.price != cancel_request.price {
            // Restore the locator since the cancel did not match.
            self.order_index.insert(cancel_request.order_id, locator);
            return Err(Box::new(CancelIDNotFoundError));
        }

        let mut removed_order: Option<Order> = None;
        let empty = match locator.side {
            OrderType::Buy => {
                let level = self.buy_side.get_mut(&locator.price).ok_or_else(|| Box::new(CancelIDNotFoundError))?;
                let order = level.remove(locator.node_index, &mut self.nodes);
                let empty = level.len == 0;
                self.free_node(locator.node_index);

                {
                    let mut account = accounts_data.index_ref(order.trader_id).lock().unwrap();
                    account.active_orders.remove(&order.order_id);
                }

                relay_server_addr.do_send(Arc::new(OutgoingMessage::CancelOccurredMessage(
                    CancelOccurredMessage {
                        side: cancel_request.side,
                        amount: order.amount,
                        symbol: self.symbol,
                        price: locator.price,
                    },
                )));

                removed_order = Some(order);
                empty
            }
            OrderType::Sell => {
                let level = self.sell_side.get_mut(&locator.price).ok_or_else(|| Box::new(CancelIDNotFoundError))?;
                let order = level.remove(locator.node_index, &mut self.nodes);
                let empty = level.len == 0;
                self.free_node(locator.node_index);

                {
                    let mut account = accounts_data.index_ref(order.trader_id).lock().unwrap();
                    account.active_orders.remove(&order.order_id);
                }

                relay_server_addr.do_send(Arc::new(OutgoingMessage::CancelOccurredMessage(
                    CancelOccurredMessage {
                        side: cancel_request.side,
                        amount: order.amount,
                        symbol: self.symbol,
                        price: locator.price,
                    },
                )));

                removed_order = Some(order);
                empty
            }
        };

        if empty {
            match locator.side {
                OrderType::Buy => {
                    self.buy_side.remove(&locator.price);
                    self.update_best_bid_after_remove(locator.price);
                }
                OrderType::Sell => {
                    self.sell_side.remove(&locator.price);
                    self.update_best_ask_after_remove(locator.price);
                }
            }
        }

        // Return the actual canceled order so that risk releases and the client
        // confirmation reflect the order's remaining quantity.
        Ok(removed_order.unwrap())
    }

    pub fn handle_incoming_order_request(
        &mut self,
        new_order_request: OrderRequest,
        accounts_data: &crate::config::GlobalAccountState,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        order_id: OrderID,
        start_time: &web::Data<SystemTime>,
    ) -> Result<Order, Box<dyn std::error::Error>> {
        match new_order_request.order_type {
            OrderType::Buy => self.handle_incoming_buy(
                new_order_request,
                accounts_data,
                relay_server_addr,
                order_id,
                start_time,
            ),
            OrderType::Sell => self.handle_incoming_sell(
                new_order_request,
                accounts_data,
                relay_server_addr,
                order_id,
                start_time,
            ),
        }
    }

    fn handle_incoming_sell(
        &mut self,
        mut sell_order: OrderRequest,
        accounts_data: &crate::config::GlobalAccountState,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        order_id: OrderID,
        start_time: &web::Data<SystemTime>,
    ) -> Result<Order, Box<dyn std::error::Error>> {
        let orig_amt = sell_order.amount;

        while sell_order.amount > 0 {
            let best_bid_price = match self.best_bid {
                Some(p) => p,
                None => break,
            };
            if best_bid_price < sell_order.price {
                break;
            }

            let (resting_order_id, buy_trader_id, resting_amount, resting_node) = {
                let level = self.buy_side.get(&best_bid_price).unwrap();
                let node_idx = level.head.unwrap();
                let node = &self.nodes[node_idx];
                (node.order.order_id, node.order.trader_id, node.order.amount, node_idx)
            };

            let amount_to_trade = cmp::min(sell_order.amount, resting_amount);

            self.handle_fill_event(
                accounts_data,
                Arc::new(Fill {
                    sell_trader_id: sell_order.trader_id,
                    buy_trader_id,
                    symbol: self.symbol,
                    amount: amount_to_trade,
                    price: best_bid_price,
                    trade_time: 1,
                    resting_side: OrderType::Buy,
                }),
                relay_server_addr,
                resting_order_id,
                order_id,
                start_time,
                resting_node,
            );

            sell_order.amount -= amount_to_trade;

            let fully_filled = {
                let level = self.buy_side.get_mut(&best_bid_price).unwrap();
                level.total_amount -= amount_to_trade;
                self.nodes[resting_node].order.amount -= amount_to_trade;
                if self.nodes[resting_node].order.amount == 0 {
                    level.pop_front(&mut self.nodes);
                    true
                } else {
                    false
                }
            };

            if fully_filled {
                self.order_index.remove(resting_order_id);
                let is_empty = {
                    let level = self.buy_side.get_mut(&best_bid_price).unwrap();
                    level.len == 0
                };
                if is_empty {
                    self.buy_side.remove(&best_bid_price);
                    self.update_best_bid_after_remove(best_bid_price);
                }
            }
        }

        if sell_order.amount > 0 {
            let resting_order = self.add_order_to_book(sell_order, order_id);

            {
                let mut account = accounts_data
                    .index_ref(sell_order.trader_id)
                    .lock()
                    .unwrap();
                account.active_orders.insert(resting_order.order_id, resting_order);
            }

            relay_server_addr.do_send(Arc::new(OutgoingMessage::NewRestingOrderMessage(
                NewRestingOrderMessage {
                    side: OrderType::Sell,
                    amount: resting_order.amount,
                    symbol: resting_order.symbol,
                    price: resting_order.price,
                },
            )));
        }

        Ok(Order {
            order_id,
            trader_id: sell_order.trader_id,
            symbol: sell_order.symbol,
            amount: orig_amt,
            price: sell_order.price,
            order_type: OrderType::Sell,
        })
    }

    fn handle_incoming_buy(
        &mut self,
        mut buy_order: OrderRequest,
        accounts_data: &crate::config::GlobalAccountState,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        order_id: OrderID,
        start_time: &web::Data<SystemTime>,
    ) -> Result<Order, Box<dyn std::error::Error>> {
        let orig_amt = buy_order.amount;

        while buy_order.amount > 0 {
            let best_ask_price = match self.best_ask {
                Some(p) => p,
                None => break,
            };
            if best_ask_price > buy_order.price {
                break;
            }

            let (resting_order_id, sell_trader_id, resting_amount, resting_node) = {
                let level = self.sell_side.get(&best_ask_price).unwrap();
                let node_idx = level.head.unwrap();
                let node = &self.nodes[node_idx];
                (node.order.order_id, node.order.trader_id, node.order.amount, node_idx)
            };

            let amount_to_trade = cmp::min(buy_order.amount, resting_amount);

            self.handle_fill_event(
                accounts_data,
                Arc::new(Fill {
                    sell_trader_id,
                    buy_trader_id: buy_order.trader_id,
                    symbol: self.symbol,
                    amount: amount_to_trade,
                    price: best_ask_price,
                    trade_time: 1,
                    resting_side: OrderType::Sell,
                }),
                relay_server_addr,
                order_id,
                resting_order_id,
                start_time,
                resting_node,
            );

            // Refund the price improvement on aggressive buys: margin was
            // reserved at buy_order.price but the fill happened at best_ask_price.
            if best_ask_price < buy_order.price && !buy_order.trader_id.is_price_enforcer() {
                let refund = (buy_order.price - best_ask_price) * amount_to_trade;
                accounts_data
                    .index_ref(buy_order.trader_id)
                    .lock()
                    .unwrap()
                    .net_cents_balance += refund;
            }

            buy_order.amount -= amount_to_trade;

            let fully_filled = {
                let level = self.sell_side.get_mut(&best_ask_price).unwrap();
                level.total_amount -= amount_to_trade;
                self.nodes[resting_node].order.amount -= amount_to_trade;
                if self.nodes[resting_node].order.amount == 0 {
                    level.pop_front(&mut self.nodes);
                    true
                } else {
                    false
                }
            };

            if fully_filled {
                self.order_index.remove(resting_order_id);
                let is_empty = {
                    let level = self.sell_side.get_mut(&best_ask_price).unwrap();
                    level.len == 0
                };
                if is_empty {
                    self.sell_side.remove(&best_ask_price);
                    self.update_best_ask_after_remove(best_ask_price);
                }
            }
        }

        if buy_order.amount > 0 {
            let resting_order = self.add_order_to_book(buy_order, order_id);

            {
                let mut account = accounts_data
                    .index_ref(buy_order.trader_id)
                    .lock()
                    .unwrap();
                account.active_orders.insert(resting_order.order_id, resting_order);
            }

            relay_server_addr.do_send(Arc::new(OutgoingMessage::NewRestingOrderMessage(
                NewRestingOrderMessage {
                    side: OrderType::Buy,
                    amount: resting_order.amount,
                    symbol: resting_order.symbol,
                    price: resting_order.price,
                },
            )));
        }

        Ok(Order {
            order_id,
            trader_id: buy_order.trader_id,
            symbol: buy_order.symbol,
            amount: orig_amt,
            price: buy_order.price,
            order_type: OrderType::Buy,
        })
    }

    fn handle_fill_event(
        &mut self,
        accounts_data: &GlobalAccountState,
        fill_event: Arc<Fill>,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        buy_trader_order_id: OrderID,
        sell_trader_order_id: OrderID,
        start_time: &web::Data<SystemTime>,
        _resting_node: usize,
    ) {
        let cent_value = fill_event.amount * fill_event.price;
        let time = start_time.elapsed().unwrap().as_secs();

        self.price_history.push((
            time,
            fill_event.price.try_into().unwrap(),
            fill_event.amount.try_into().unwrap(),
        ));

        let mut buy_trader = accounts_data
            .index_ref(fill_event.buy_trader_id)
            .lock()
            .unwrap();

        if !buy_trader.trader_id.is_price_enforcer() {
            *buy_trader.asset_balances.index_ref_mut(&fill_event.symbol) +=
                <usize as TryInto<i64>>::try_into(fill_event.amount).unwrap();
            *buy_trader.net_asset_balances.index_ref_mut(&fill_event.symbol) +=
                <usize as TryInto<i64>>::try_into(fill_event.amount).unwrap();
            buy_trader.cents_balance -= cent_value;
        }

        // Update the buyer's active order (partial fill or full fill).
        if let Some(order) = buy_trader.active_orders.get_mut(&buy_trader_order_id) {
            if fill_event.amount >= order.amount {
                buy_trader.active_orders.remove(&buy_trader_order_id);
            } else {
                order.amount -= fill_event.amount;
            }
        }

        let buy_fill = JsonPayload(Arc::from(serde_json::to_string(&OutgoingMessage::OrderFillMessage(
            OrderFillMessage {
                order_id: buy_trader_order_id,
                amount_filled: fill_event.amount,
                price: fill_event.price,
            },
        )).unwrap()));
        buy_trader.push_fill(buy_fill);
        drop(buy_trader);

        let mut sell_trader = accounts_data
            .index_ref(fill_event.sell_trader_id)
            .lock()
            .unwrap();

        if !sell_trader.trader_id.is_price_enforcer() {
            *sell_trader.asset_balances.index_ref_mut(&fill_event.symbol) -=
                <usize as TryInto<i64>>::try_into(fill_event.amount).unwrap();
            sell_trader.cents_balance += cent_value;
            sell_trader.net_cents_balance += cent_value;
        }

        // Update the seller's active order (partial fill or full fill).
        if let Some(order) = sell_trader.active_orders.get_mut(&sell_trader_order_id) {
            if fill_event.amount >= order.amount {
                sell_trader.active_orders.remove(&sell_trader_order_id);
            } else {
                order.amount -= fill_event.amount;
            }
        }

        let sell_fill = JsonPayload(Arc::from(serde_json::to_string(&OutgoingMessage::OrderFillMessage(
            OrderFillMessage {
                order_id: sell_trader_order_id,
                amount_filled: fill_event.amount,
                price: fill_event.price,
            },
        )).unwrap()));
        sell_trader.push_fill(sell_fill);
        drop(sell_trader);

        relay_server_addr.do_send(Arc::new(OutgoingMessage::TradeOccurredMessage(
            TradeOccurredMessage {
                amount: fill_event.amount,
                symbol: fill_event.symbol,
                price: fill_event.price,
                resting_side: fill_event.resting_side,
                time,
            },
        )));

        trace!(
            "{:?} sells to {:?}: {:?} lots of {:?} @ ${:?}",
            fill_event.sell_trader_id,
            fill_event.buy_trader_id,
            fill_event.amount,
            fill_event.symbol,
            fill_event.price
        );
    }

    pub fn level_len(&self, side: OrderType, price: Price) -> usize {
        match side {
            OrderType::Buy => self.buy_side.get(&price).map_or(0, |l| l.len),
            OrderType::Sell => self.sell_side.get(&price).map_or(0, |l| l.len),
        }
    }

    pub fn get_book_state(&self) -> String {
        let mut all_prices: std::collections::BTreeSet<Price> = std::collections::BTreeSet::new();
        all_prices.extend(self.buy_side.keys().copied());
        all_prices.extend(self.sell_side.keys().copied());

        let mut ret_string = String::from("{[");
        for price in &all_prices {
            let buy_vol = self.buy_side.get(price).map_or(0, |l| l.total_amount);
            let sell_vol = self.sell_side.get(price).map_or(0, |l| l.total_amount);
            ret_string.push_str(&format!(
                "{{price:{},sellVolume:{},buyVolume:{}}},",
                price, sell_vol, buy_vol
            ));
        }
        ret_string.push_str("]}");
        ret_string
    }

    pub fn print_book_state(&self) {
        println!("Orderbook for {:?}", self.symbol);
        let mut all_prices: std::collections::BTreeSet<Price> = std::collections::BTreeSet::new();
        all_prices.extend(self.buy_side.keys().copied());
        all_prices.extend(self.sell_side.keys().copied());

        for price in &all_prices {
            let buy_vol = self.buy_side.get(price).map_or(0, |l| l.total_amount);
            let sell_vol = self.sell_side.get(price).map_or(0, |l| l.total_amount);
            let mut s = String::new();
            for _ in 0..sell_vol {
                s.push('S');
            }
            for _ in 0..buy_vol {
                s.push('B');
            }
            println!("${}: {}", price, s);
        }
    }
}

pub fn quickstart_order_book(symbol: config::TickerSymbol) -> OrderBook {
    OrderBook {
        symbol,
        buy_side: BTreeMap::new(),
        sell_side: BTreeMap::new(),
        nodes: Vec::with_capacity(1024),
        free_nodes: Vec::with_capacity(1024),
        order_index: OrderIndex::default(),
        price_history: Vec::new(),
        best_bid: None,
        best_ask: None,
    }
}
