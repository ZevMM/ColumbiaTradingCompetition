use crate::api_messages::{
    CancelIDNotFoundError, CancelOccurredMessage, CancelRequest, NewRestingOrderMessage,
    OrderFillMessage, OrderRequest, OutgoingMessage, TradeOccurredMessage,
};
use crate::config::{self, GlobalAccountState};
use crate::connection_server;
use std::collections::{BTreeMap, HashMap, VecDeque};
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


#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum OrderType {
    Buy,
    Sell,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct OrderBook {
    pub symbol: config::TickerSymbol,
    pub buy_side: BTreeMap<Price, VecDeque<Order>>,
    pub sell_side: BTreeMap<Price, VecDeque<Order>>,
    pub order_index: HashMap<OrderID, Price>,
    pub price_history: Vec<(u64, u16, u16)>,
}

impl Serialize for OrderBook {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("OrderBook", 4)?;
        s.serialize_field("symbol", &self.symbol)?;
        let buy_volumes: BTreeMap<Price, usize> = self
            .buy_side
            .iter()
            .map(|(&p, q)| (p, q.iter().map(|o| o.amount).sum()))
            .collect();
        s.serialize_field("buy_side", &buy_volumes)?;
        let sell_volumes: BTreeMap<Price, usize> = self
            .sell_side
            .iter()
            .map(|(&p, q)| (p, q.iter().map(|o| o.amount).sum()))
            .collect();
        s.serialize_field("sell_side", &sell_volumes)?;
        s.serialize_field("price_history", &self.price_history)?;
        s.end()
    }
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

impl OrderBook {
    fn add_order_to_book(&mut self, new_order_request: OrderRequest, order_id: OrderID) -> Order {
        let new_order = Order {
            order_id,
            trader_id: new_order_request.trader_id,
            symbol: new_order_request.symbol,
            amount: new_order_request.amount,
            price: new_order_request.price,
            order_type: new_order_request.order_type,
        };
        match new_order.order_type {
            OrderType::Buy => {
                self.buy_side
                    .entry(new_order.price)
                    .or_insert_with(VecDeque::new)
                    .push_back(new_order);
            }
            OrderType::Sell => {
                self.sell_side
                    .entry(new_order.price)
                    .or_insert_with(VecDeque::new)
                    .push_back(new_order);
            }
        }
        self.order_index.insert(order_id, new_order.price);
        new_order
    }

    pub fn handle_incoming_cancel_request(
        &mut self,
        cancel_request: CancelRequest,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        accounts_data: &GlobalAccountState,
    ) -> Result<Order, Box<dyn std::error::Error>> {
        let price = match self.order_index.get(&cancel_request.order_id) {
            Some(&p) => p,
            None => return Err(Box::new(CancelIDNotFoundError)),
        };

        // Extract the canceled order and whether the level is now empty,
        // releasing the BTreeMap borrow before any subsequent mutations.
        let result = match cancel_request.side {
            OrderType::Buy => {
                let maybe = self.buy_side.get_mut(&price).and_then(|q| {
                    q.iter()
                        .position(|o| o.order_id == cancel_request.order_id)
                        .map(|idx| {
                            let order = q.remove(idx).unwrap();
                            let empty = q.is_empty();
                            (order, empty)
                        })
                });
                if let Some((_, true)) = &maybe {
                    self.buy_side.remove(&price);
                }
                maybe
            }
            OrderType::Sell => {
                let maybe = self.sell_side.get_mut(&price).and_then(|q| {
                    q.iter()
                        .position(|o| o.order_id == cancel_request.order_id)
                        .map(|idx| {
                            let order = q.remove(idx).unwrap();
                            let empty = q.is_empty();
                            (order, empty)
                        })
                });
                if let Some((_, true)) = &maybe {
                    self.sell_side.remove(&price);
                }
                maybe
            }
        };

        match result {
            Some((canceled_order, _)) => {
                self.order_index.remove(&cancel_request.order_id);

                {
                    let mut account = accounts_data
                        .index_ref(canceled_order.trader_id)
                        .lock()
                        .unwrap();
                    account
                        .active_orders
                        .retain(|&x| x.order_id != canceled_order.order_id);
                }

                relay_server_addr.do_send(Arc::new(OutgoingMessage::CancelOccurredMessage(
                    CancelOccurredMessage {
                        side: cancel_request.side,
                        amount: canceled_order.amount,
                        symbol: self.symbol,
                        price,
                    },
                )));

                Ok(canceled_order)
            }
            None => Err(Box::new(CancelIDNotFoundError)),
        }
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

        // Match against resting buy orders in descending price order.
        while sell_order.amount > 0 {
            let best_bid_price = match self.buy_side.keys().next_back() {
                Some(&p) => p,
                None => break,
            };
            if best_bid_price < sell_order.price {
                break;
            }

            // Extract resting order data before any mutable operations.
            let (resting_order_id, buy_trader_id, resting_amount) = {
                let queue = self.buy_side.get(&best_bid_price).unwrap();
                let front = queue.front().unwrap();
                (front.order_id, front.trader_id, front.amount)
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
            );

            sell_order.amount -= amount_to_trade;

            let fully_filled = {
                let queue = self.buy_side.get_mut(&best_bid_price).unwrap();
                let front = queue.front_mut().unwrap();
                front.amount -= amount_to_trade;
                front.amount == 0
            };

            if fully_filled {
                {
                    let mut counter_party = accounts_data
                        .index_ref(buy_trader_id)
                        .lock()
                        .unwrap();
                    counter_party
                        .active_orders
                        .retain(|&x| x.order_id != resting_order_id);
                }
                self.order_index.remove(&resting_order_id);
                let is_empty = {
                    let queue = self.buy_side.get_mut(&best_bid_price).unwrap();
                    queue.pop_front();
                    queue.is_empty()
                };
                if is_empty {
                    self.buy_side.remove(&best_bid_price);
                }
            } else {
                let mut counter_party = accounts_data
                    .index_ref(buy_trader_id)
                    .lock()
                    .unwrap();
                if let Some(order) = counter_party
                    .active_orders
                    .iter_mut()
                    .find(|x| x.order_id == resting_order_id)
                {
                    order.amount -= amount_to_trade;
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
                account.active_orders.push(resting_order);
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

        // Match against resting sell orders in ascending price order.
        while buy_order.amount > 0 {
            let best_ask_price = match self.sell_side.keys().next() {
                Some(&p) => p,
                None => break,
            };
            if best_ask_price > buy_order.price {
                break;
            }

            // Extract resting order data before any mutable operations.
            let (resting_order_id, sell_trader_id, resting_amount) = {
                let queue = self.sell_side.get(&best_ask_price).unwrap();
                let front = queue.front().unwrap();
                (front.order_id, front.trader_id, front.amount)
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
            );

            buy_order.amount -= amount_to_trade;

            let fully_filled = {
                let queue = self.sell_side.get_mut(&best_ask_price).unwrap();
                let front = queue.front_mut().unwrap();
                front.amount -= amount_to_trade;
                front.amount == 0
            };

            if fully_filled {
                {
                    let mut counter_party = accounts_data
                        .index_ref(sell_trader_id)
                        .lock()
                        .unwrap();
                    counter_party
                        .active_orders
                        .retain(|&x| x.order_id != resting_order_id);
                }
                self.order_index.remove(&resting_order_id);
                let is_empty = {
                    let queue = self.sell_side.get_mut(&best_ask_price).unwrap();
                    queue.pop_front();
                    queue.is_empty()
                };
                if is_empty {
                    self.sell_side.remove(&best_ask_price);
                }
            } else {
                let mut counter_party = accounts_data
                    .index_ref(sell_trader_id)
                    .lock()
                    .unwrap();
                if let Some(order) = counter_party
                    .active_orders
                    .iter_mut()
                    .find(|x| x.order_id == resting_order_id)
                {
                    order.amount -= amount_to_trade;
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
                account.active_orders.push(resting_order);
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
            *buy_trader
                .asset_balances
                .index_ref(&fill_event.symbol)
                .lock()
                .unwrap() += <usize as TryInto<i64>>::try_into(fill_event.amount).unwrap();
            *buy_trader
                .net_asset_balances
                .index_ref(&fill_event.symbol)
                .lock()
                .unwrap() += <usize as TryInto<i64>>::try_into(fill_event.amount).unwrap();
            buy_trader.cents_balance -= cent_value;
        }

        let buy_trader_fill_msg = Arc::new(OrderFillMessage {
            order_id: buy_trader_order_id,
            amount_filled: fill_event.amount,
            price: fill_event.price,
        });
        buy_trader.push_fill(buy_trader_fill_msg);
        drop(buy_trader);

        let mut sell_trader = accounts_data
            .index_ref(fill_event.sell_trader_id)
            .lock()
            .unwrap();

        if !sell_trader.trader_id.is_price_enforcer() {
            *sell_trader
                .asset_balances
                .index_ref(&fill_event.symbol)
                .lock()
                .unwrap() -= <usize as TryInto<i64>>::try_into(fill_event.amount).unwrap();
            sell_trader.cents_balance += cent_value;
            sell_trader.net_cents_balance += cent_value;
        }

        let sell_trader_fill_msg = Arc::new(OrderFillMessage {
            order_id: sell_trader_order_id,
            amount_filled: fill_event.amount,
            price: fill_event.price,
        });
        sell_trader.push_fill(sell_trader_fill_msg);

        relay_server_addr.do_send(Arc::new(OutgoingMessage::TradeOccurredMessage(
            TradeOccurredMessage {
                amount: fill_event.amount,
                symbol: fill_event.symbol,
                price: fill_event.price,
                resting_side: fill_event.resting_side,
                time,
            },
        )));

        println!(
            "{:?} sells to {:?}: {:?} lots of {:?} @ ${:?}",
            fill_event.sell_trader_id,
            fill_event.buy_trader_id,
            fill_event.amount,
            fill_event.symbol,
            fill_event.price
        );
    }

    pub fn get_book_state(&self) -> String {
        let mut all_prices: std::collections::BTreeSet<Price> = std::collections::BTreeSet::new();
        all_prices.extend(self.buy_side.keys().copied());
        all_prices.extend(self.sell_side.keys().copied());

        let mut ret_string = String::from("{[");
        for price in &all_prices {
            let buy_vol: usize = self
                .buy_side
                .get(price)
                .map_or(0, |q| q.iter().map(|o| o.amount).sum());
            let sell_vol: usize = self
                .sell_side
                .get(price)
                .map_or(0, |q| q.iter().map(|o| o.amount).sum());
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
            let buy_vol: usize = self
                .buy_side
                .get(price)
                .map_or(0, |q| q.iter().map(|o| o.amount).sum());
            let sell_vol: usize = self
                .sell_side
                .get(price)
                .map_or(0, |q| q.iter().map(|o| o.amount).sum());
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
        order_index: HashMap::new(),
        price_history: Vec::new(),
    }
}
