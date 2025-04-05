use crate::api_messages::{CancelIDNotFoundError, CancelOccurredMessage, CancelRequest, NewRestingOrderMessage, OrderFillMessage, OrderRequest, OutgoingMessage, TradeOccurredMessage};
use crate::config::{self, GlobalAccountState};
use crate::connection_server;
use std::sync::Arc;
use std::time::SystemTime;
use actix::prelude::*;
use actix_web::web;
use std::cmp;
use core::sync::atomic::AtomicUsize;
pub type OrderID = usize;
pub type Price = usize;
pub type TraderId = config::TraderId;
use serde::{Deserialize, Serialize};
extern crate env_logger;


#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum OrderType {
    Buy,
    Sell,
}

#[derive(Debug, Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderBook {
    /// Struct representing a double sided order book for a single product.
    // todo: add offset to allow for non 0 min prices
    pub symbol: config::TickerSymbol,
    // buy side in increasing price order
    pub buy_side_limit_levels: Vec<LimitLevel>,
    // sell side in increasing price order
    pub sell_side_limit_levels: Vec<LimitLevel>,
    current_high_buy_price: Price,
    current_low_sell_price: Price,
    pub price_history: Vec<(u64, u16, u16)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitLevel {
    /// Struct representing one price level in the orderbook, containing a vector of Orders at this price
    price: Price,
    // this is a stopgap measure to deal with sending out full orderbooks on connect.
    // TODO: write own serializer
    #[serde(skip_serializing)]
    pub orders: Vec<Order>,
    total_volume: usize,
}

#[derive(Debug, Clone, Serialize, Copy, Deserialize)]
pub struct Order {
    /// Struct representing an existing order in the order book
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
    /// Struct representing an order fill event, used to update credit limits, communicate orderbook status etc.
    pub sell_trader_id: TraderId,
    pub buy_trader_id: TraderId,
    pub amount: usize,
    pub price: Price,
    pub symbol: config::TickerSymbol,
    pub trade_time: u8,
    pub resting_side: OrderType
}

impl Message for Fill {
    type Result = ();
}

impl OrderBook {
fn add_order_to_book(
        &mut self,
        new_order_request: OrderRequest,
        order_counter: &web::Data<Arc<AtomicUsize>>,
        order_id: OrderID
    ) -> Order {
        // should add error handling if push fails
        let new_order = Order {
            order_id: order_id,
            trader_id: new_order_request.trader_id,
            symbol: new_order_request.symbol,
            amount: new_order_request.amount,
            price: new_order_request.price,
            order_type: new_order_request.order_type,
        };
        match new_order.order_type {
            OrderType::Buy => {
                if self.current_high_buy_price < new_order.price {
                    self.current_high_buy_price = new_order.price;
                };
                self.buy_side_limit_levels[new_order.price]
                    .orders
                    .push(new_order.clone());
            }
            OrderType::Sell => {
                if self.current_low_sell_price > new_order.price {
                    self.current_low_sell_price = new_order.price;
                };
                self.sell_side_limit_levels[new_order.price]
                    .orders
                    .push(new_order.clone());
            }
        }
        new_order
    }

    // should use Result instead of Option to pass up info about error if needed.
    pub fn handle_incoming_cancel_request(
        &mut self,
        cancel_request: CancelRequest,
        order_counter: &web::Data<Arc<AtomicUsize>>,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        accounts_data: &GlobalAccountState,
    ) -> Result<Order, Box<dyn std::error::Error>> {

        match cancel_request.side {
            OrderType::Buy => {
                let mut index = 0;
                while index
                    < self.buy_side_limit_levels[cancel_request.price]
                        .orders
                        .len()
                {
                    if self.buy_side_limit_levels[cancel_request.price].orders[index].order_id
                        == cancel_request.order_id
                    {
                        let canceled_order = self.buy_side_limit_levels[cancel_request.price]
                                .orders
                                .remove(index);
                        
                        let mut account = accounts_data.index_ref(canceled_order.trader_id).lock().unwrap();
                        account.active_orders.retain(|&x| x.order_id != canceled_order.order_id);
                        
                        self.buy_side_limit_levels[cancel_request.price].total_volume -= canceled_order.amount;
                        
                        relay_server_addr.do_send(Arc::new(OutgoingMessage::CancelOccurredMessage(CancelOccurredMessage{
                            side: OrderType::Buy,
                            amount: canceled_order.amount,
                            symbol: self.symbol,
                            price: cancel_request.price
                        })));
                            
                        return Ok(canceled_order);
                    }
                    index += 1;
                }
                return Err(Box::new(CancelIDNotFoundError))
            }
            OrderType::Sell => {
                let mut index = 0;
                while index
                    < self.sell_side_limit_levels[cancel_request.price]
                        .orders
                        .len()
                {
                    if self.sell_side_limit_levels[cancel_request.price].orders[index].order_id
                        == cancel_request.order_id
                    {
                        let canceled_order = 
                            self.sell_side_limit_levels[cancel_request.price]
                                .orders
                                .remove(index);
                            
                            let mut account = accounts_data.index_ref(canceled_order.trader_id).lock().unwrap();
                            account.active_orders.retain(|&x| x.order_id != canceled_order.order_id);

                            self.sell_side_limit_levels[cancel_request.price].total_volume -= canceled_order.amount;

                            relay_server_addr.do_send(Arc::new(OutgoingMessage::CancelOccurredMessage(CancelOccurredMessage{
                                side: OrderType::Sell,
                                amount: canceled_order.amount,
                                symbol: self.symbol,
                                price: cancel_request.price
                            })));
                        return Ok(canceled_order);
                    }
                    index += 1;
                }
                return Err(Box::new(CancelIDNotFoundError))
            }
        };

        

    }

    pub fn handle_incoming_order_request(
        &mut self,
        new_order_request: OrderRequest,
        accounts_data: &crate::config::GlobalAccountState,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        order_counter: &web::Data<Arc<AtomicUsize>>,
        order_id: OrderID,
        start_time: &web::Data<SystemTime>
    ) -> Result<Order, Box<dyn std::error::Error>> {
        match new_order_request.order_type {
            OrderType::Buy => self.handle_incoming_buy(
                new_order_request,
                accounts_data,
                relay_server_addr,
                order_counter,
                order_id,
                start_time
            ),
            OrderType::Sell => self.handle_incoming_sell(
                new_order_request,
                accounts_data,
                relay_server_addr,
                order_counter,
                order_id,
                start_time
            ),
        }
    }

    fn handle_incoming_sell(
        &mut self,
        mut sell_order: OrderRequest,
        accounts_data: &crate::config::GlobalAccountState,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        order_counter: &web::Data<Arc<AtomicUsize>>,
        order_id: OrderID,
        start_time: &web::Data<SystemTime>
    ) -> Result<Order, Box<dyn std::error::Error>> {

        let orig_amt = sell_order.amount;
        if sell_order.price <= self.current_high_buy_price {

            let mut current_price_level = self.current_high_buy_price;
            while (sell_order.amount > 0) & (current_price_level >= sell_order.price) {

                while (self.buy_side_limit_levels[current_price_level].orders.len() > 0)
                    & (sell_order.amount > 0)
                {
                    let trade_price =
                        self.buy_side_limit_levels[current_price_level].orders[0].price;
                    let buy_trader_id =
                        self.buy_side_limit_levels[current_price_level].orders[0].trader_id;

                    let buy_trader_order_id = self.buy_side_limit_levels[current_price_level].orders[0].order_id;

                    let amount_to_be_traded = cmp::min(
                        sell_order.amount,
                        self.buy_side_limit_levels[current_price_level].orders[0].amount,
                    );

                    self.handle_fill_event(
                        accounts_data,
                        Arc::new(Fill {
                            sell_trader_id: sell_order.trader_id,
                            buy_trader_id: buy_trader_id,
                            symbol: self.symbol,
                            amount: amount_to_be_traded,
                            price: trade_price,
                            trade_time: 1,
                            resting_side: OrderType::Buy,
                        }),
                        relay_server_addr,
                        buy_trader_order_id,
                        order_id,
                        start_time
                    );

                    sell_order.amount -= amount_to_be_traded;
                    self.buy_side_limit_levels[current_price_level].orders[0].amount -=
                        amount_to_be_traded;

                    self.buy_side_limit_levels[current_price_level].total_volume -=
                        amount_to_be_traded;

                    if self.buy_side_limit_levels[current_price_level].orders[0].amount == 0 {
                        let mut counter_party = accounts_data.index_ref(self.buy_side_limit_levels[current_price_level].orders[0].trader_id).lock().unwrap();
                        counter_party.active_orders.retain(|&x| x.order_id != self.buy_side_limit_levels[current_price_level].orders[0].order_id);
                        
                        self.buy_side_limit_levels[current_price_level]
                            .orders
                            .remove(0);
                    } else {
                        let mut counter_party = accounts_data.index_ref(self.buy_side_limit_levels[current_price_level].orders[0].trader_id).lock().unwrap();
                        let to_reduce = counter_party.active_orders.iter_mut().find(|&&mut x| x.order_id == self.buy_side_limit_levels[current_price_level].orders[0].order_id);
                        to_reduce.unwrap().amount -= amount_to_be_traded;
                    }

                }
                current_price_level = match current_price_level.checked_sub(1) {
                    Some(new_level) => new_level,
                    None => break,
                };
            }
            // To do: find a more elegant way to avoid "skipping" price levels on the way down.
            current_price_level += 1;

            while current_price_level > 0 {
                if self.buy_side_limit_levels[current_price_level].orders.len() > 0 {
                    self.current_high_buy_price = current_price_level;
                    break;
                }
                current_price_level -= 1;
            }
            self.current_high_buy_price = current_price_level;
        }
        // will be changed to beam out book state to subscribers

        if sell_order.amount > 0 {
            let resting_order = self.add_order_to_book(sell_order, order_counter, order_id);
            
            let mut account = accounts_data.index_ref(sell_order.trader_id).lock().unwrap();
            account.active_orders.push(resting_order);
            
            self.sell_side_limit_levels[sell_order.price].total_volume += sell_order.amount;

            relay_server_addr.do_send(Arc::new(OutgoingMessage::NewRestingOrderMessage(NewRestingOrderMessage{
                side: OrderType::Sell,
                amount: resting_order.amount,
                symbol: resting_order.symbol,
                price: resting_order.price
            })));

            return Ok(Order {
                order_id: order_id,
                trader_id: sell_order.trader_id,
                symbol: sell_order.symbol,
                amount: orig_amt,
                price: sell_order.price,
                order_type: OrderType::Sell,
            });
        } else {
            return Ok(Order {
                order_id: order_id,
                trader_id: sell_order.trader_id,
                symbol: sell_order.symbol,
                amount: orig_amt,
                price: sell_order.price,
                order_type: OrderType::Sell,
            });
        }
    }
    fn handle_incoming_buy(
        &mut self,
        mut buy_order: OrderRequest,
        accounts_data: &crate::config::GlobalAccountState,
        relay_server_addr: &web::Data<Addr<connection_server::Server>>,
        order_counter: &web::Data<Arc<AtomicUsize>>,
        // this should be folded into OrderRequest eventually
        order_id: OrderID,
        start_time: &web::Data<SystemTime>
    ) -> Result<Order, Box<dyn std::error::Error>> {

        let orig_amt = buy_order.amount;
        if buy_order.price >= self.current_low_sell_price {
            let mut current_price_level = self.current_low_sell_price;
            while (buy_order.amount > 0) & (current_price_level <= buy_order.price) {
                // let mut order_index = 0;
                while (0 < self.sell_side_limit_levels[current_price_level]
                    .orders
                    .len())
                    & (buy_order.amount > 0)
                {
                    let trade_price =
                        self.sell_side_limit_levels[current_price_level].orders[0].price;
                    let sell_trader_id =
                        self.sell_side_limit_levels[current_price_level].orders[0].trader_id;

                    let amount_to_be_traded = cmp::min(
                        buy_order.amount,
                        self.sell_side_limit_levels[current_price_level].orders[0].amount,
                    );

                    self.handle_fill_event(
                        accounts_data,
                        Arc::new(Fill {
                            sell_trader_id: sell_trader_id,
                            buy_trader_id: buy_order.trader_id,
                            symbol: self.symbol,
                            amount: amount_to_be_traded,
                            price: trade_price,
                            trade_time: 1,
                            resting_side: OrderType::Sell,
                        }),
                        relay_server_addr,
                        order_id,
                        self.sell_side_limit_levels[current_price_level].orders[0].order_id,
                        start_time
                    );

                    // TODO: create "sell" function that can handle calls to allocate credit etc.
                    // also removing from the front seems pretty inefficient,
                    buy_order.amount -= amount_to_be_traded;
                    self.sell_side_limit_levels[current_price_level].orders[0].amount -=
                        amount_to_be_traded;
                    self.sell_side_limit_levels[current_price_level].total_volume -=
                        amount_to_be_traded;


                    if self.sell_side_limit_levels[current_price_level].orders[0].amount == 0 {
                        let mut counter_party = accounts_data.index_ref(self.sell_side_limit_levels[current_price_level].orders[0].trader_id).lock().unwrap();
                        counter_party.active_orders.retain(|&x| x.order_id != self.sell_side_limit_levels[current_price_level].orders[0].order_id);
                        self.sell_side_limit_levels[current_price_level]
                            .orders
                            .remove(0);
                    } else {
                        let mut counter_party = accounts_data.index_ref(self.sell_side_limit_levels[current_price_level].orders[0].trader_id).lock().unwrap();
                        let to_reduce = counter_party.active_orders.iter_mut().find(|&&mut x| x.order_id == self.sell_side_limit_levels[current_price_level].orders[0].order_id);
                        to_reduce.unwrap().amount -= amount_to_be_traded;
                    }
                }

                current_price_level += 1;
            }
            current_price_level -= 1;
            // in the event that a price level has been completely bought, update lowest sell price
            while current_price_level < self.sell_side_limit_levels.len() {
                if self.sell_side_limit_levels[current_price_level]
                    .orders
                    .len()
                    > 0
                {
                    self.current_low_sell_price = current_price_level;
                    break;
                }
                current_price_level += 1;
            }
            self.current_low_sell_price = current_price_level;
        }
        // will be changed to beam out book state to subscribers

        if buy_order.amount > 0 {
            let resting_order = self.add_order_to_book(buy_order, order_counter, order_id);
            let mut account = accounts_data.index_ref(buy_order.trader_id).lock().unwrap();
            account.active_orders.push(resting_order);

            self.buy_side_limit_levels[buy_order.price].total_volume += buy_order.amount;

            // issue async is the culprit hanging up performance
            relay_server_addr.do_send(Arc::new(OutgoingMessage::NewRestingOrderMessage(NewRestingOrderMessage{
                side: OrderType::Buy,
                amount: resting_order.amount,
                symbol: resting_order.symbol,
                price: resting_order.price
            })));

            return Ok(Order {
                order_id: order_id,
                trader_id: buy_order.trader_id,
                symbol: buy_order.symbol,
                amount: orig_amt,
                price: buy_order.price,
                order_type: OrderType::Buy,
            });
        } else {
            // order was filled before it rested on the book, order_id = 0 is special
            return Ok(Order {
                order_id: order_id,
                trader_id: buy_order.trader_id,
                symbol: buy_order.symbol,
                amount: orig_amt,
                price: buy_order.price,
                order_type: OrderType::Buy,
            });
        }
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

        let cent_value = &fill_event.amount * &fill_event.price;
        let time = start_time.elapsed().unwrap().as_secs();
        
        self.price_history.push((time, fill_event.price.try_into().unwrap(), fill_event.amount.try_into().unwrap()));

        let mut buy_trader = accounts_data.index_ref(fill_event.buy_trader_id).lock().unwrap();

        if buy_trader.trader_id != TraderId::Price_Enforcer {
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
        
        let buy_trader_order_fill_msg = Arc::new(OrderFillMessage {
            order_id: buy_trader_order_id,
            amount_filled: fill_event.amount,
            price: fill_event.price,
        });

        buy_trader.push_fill(buy_trader_order_fill_msg);

        drop(buy_trader);

        let mut sell_trader = accounts_data.index_ref(fill_event.sell_trader_id).lock().unwrap();

        // would need to iterate over all traders and clone once per.
        if sell_trader.trader_id != TraderId::Price_Enforcer {
            *sell_trader
                .asset_balances
                .index_ref(&fill_event.symbol)
                .lock()
                .unwrap() -= <usize as TryInto<i64>>::try_into(fill_event.amount).unwrap();


            sell_trader.cents_balance += cent_value;
            sell_trader.net_cents_balance += cent_value;
        }
        
        let sell_trader_order_fill_msg = Arc::new(OrderFillMessage {
            order_id: sell_trader_order_id,
            amount_filled: fill_event.amount,
            price: fill_event.price,
        });
        
        sell_trader.push_fill(sell_trader_order_fill_msg);


        let trade_occurred_message = Arc::new(OutgoingMessage::TradeOccurredMessage(
            TradeOccurredMessage {
                amount: fill_event.amount, 
                symbol: fill_event.symbol,
                price: fill_event.price,
                resting_side: fill_event.resting_side,
                time: time
            }
        ));

        relay_server_addr.do_send(trade_occurred_message);

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
        let mut ret_string = String::from("{[");
        for price_level_index in 0..self.buy_side_limit_levels.len() {
            let mut outstanding_sell_orders: usize = 0;
            let mut outstanding_buy_orders: usize = 0;
            for order in self.sell_side_limit_levels[price_level_index].orders.iter() {
                outstanding_sell_orders += order.amount;
            }
            for order in self.buy_side_limit_levels[price_level_index].orders.iter() {
                outstanding_buy_orders += order.amount;
            }

            let limlevstr = format!(
                "{{sellVolume:{},buyVolume:{}}},",
                outstanding_sell_orders, outstanding_buy_orders
            );
            ret_string.push_str(&limlevstr);

        }
        ret_string.push_str("]}");
        return ret_string;
    }

    pub fn print_book_state(&self) {
        println!("Orderbook for {:?}", self.symbol);
        for price_level_index in 0..self.buy_side_limit_levels.len() {
            let mut outstanding_sell_orders: usize = 0;
            let mut outstanding_buy_orders: usize = 0;
            for order in self.sell_side_limit_levels[price_level_index].orders.iter() {
                outstanding_sell_orders += order.amount;
            }
            for order in self.buy_side_limit_levels[price_level_index].orders.iter() {
                outstanding_buy_orders += order.amount;
            }
            let mut string_out = String::from("");
            for _ in 0..outstanding_sell_orders {
                string_out = string_out + "S"
            }
            for _ in 0..outstanding_buy_orders {
                string_out = string_out + "B"
            }
            println!(
                "${}: {}",
                self.buy_side_limit_levels[price_level_index].price, string_out
            );
        }
    }
}

pub fn quickstart_order_book(
    symbol: config::TickerSymbol,
    min_price: Price,
    max_price: Price,
    capacity_per_lim_lev: usize,
) -> OrderBook {
    OrderBook {
        symbol: config::TickerSymbol::from(symbol),
        buy_side_limit_levels: (min_price..max_price)
            .map(|x| LimitLevel {
                price: x,
                orders: Vec::with_capacity(capacity_per_lim_lev),
                total_volume: 0,
            })
            .collect(),
        sell_side_limit_levels: (min_price..max_price)
            .map(|x| LimitLevel {
                price: x,
                orders: Vec::with_capacity(capacity_per_lim_lev),
                total_volume: 0,
            })
            .collect(),
        current_high_buy_price: min_price,
        current_low_sell_price: max_price,
        price_history: Vec::new(),
    }
}
