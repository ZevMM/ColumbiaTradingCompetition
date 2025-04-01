use actix::prelude::*;
use actix_web::web::Bytes;
use actix_web::Error;
use actix_web_actors::ws;
use log::info;
use plotters::coord::types;
use serde_json::json;
use std::env;
use std::f32::consts::E;
use std::fmt::format;
use std::net::Ipv4Addr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use uuid::serde;

use strum::IntoEnumIterator; // 0.17.1
use strum_macros::EnumIter; // 0.17.1

use actix_broker::{ArbiterBroker, Broker, BrokerIssue, BrokerSubscribe, SystemBroker};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(4);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
use crate::api_messages::{
    self, CancelConfirmMessage, CancelErrorMessage, CancelRequest, IncomingMessage,
    OrderConfirmMessage, OrderFillMessage, OrderPlaceErrorMessage, OrderPlaceResponse,
    OrderRequest, OutgoingMessage, TradeOccurredMessage
};
use crate::message_types::{CloseMessage, GameEndMessage, GameStartedMessage, OpenMessage};
use crate::orderbook::{Fill, TraderId};
use crate::websockets::ws::CloseCode::Policy;
use crate::websockets::ws::CloseReason;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;
extern crate env_logger;

use queues::IsQueue;

use actix::prelude::*;

use std::any::type_name;

// mod orderbook;
// mod accounts;
// mod macro_calls;
// mod websockets;

pub use crate::accounts::TraderAccount;
// pub use crate::orderbook::TickerSymbol;
pub use crate::accounts::quickstart_trader_account;
use crate::config::TraderIp;
pub use crate::orderbook::quickstart_order_book;
pub use crate::orderbook::OrderBook;
pub use crate::orderbook::OrderType;
use crate::{config, orderbook, GlobalState};

use crate::config::AssetBalances;
use crate::config::TickerSymbol;

use crate::config::GlobalAccountState;
use crate::config::GlobalOrderBookState;

use ::serde::{de, Deserialize, Serialize};

pub fn add_order<'a>(
    order_request: OrderRequest,
    data: &crate::config::GlobalOrderBookState,
    accounts_data: &crate::config::GlobalAccountState,
    relay_server_addr: &web::Data<Addr<crate::connection_server::Server>>,
    order_counter: &web::Data<Arc<AtomicUsize>>,
    start_time: &web::Data<SystemTime>
) -> OrderPlaceResponse<'a> {
    println!("Add Order Triggered!");

    let order_request_inner = order_request;
    let symbol = &order_request_inner.symbol;

    // Todo: refactor into match statement, put into actix guard?
    if (order_request_inner.order_type == crate::orderbook::OrderType::Buy) {
        // ISSUE: This should decrement cents_balance to avoid racing to place two orders before updating cents_balance
        // check if current cash balance - outstanding orders supports order
        // nevermind, as long as I acquire and hold a lock during the entire order placement attempt, it should be safe
        let cent_value = &order_request_inner.amount * &order_request_inner.price;
        if ((accounts_data
            .index_ref(order_request_inner.trader_id)
            .lock()
            .unwrap()
            .net_cents_balance
            < cent_value)
            && order_request_inner.trader_id != TraderId::Price_Enforcer)
        {
            return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage{
                side: order_request_inner.order_type,
                price: order_request_inner.price,
                symbol: order_request_inner.symbol,
                error_details: "Error Placing Order: The total value of order is greater than current account balance"
            });
        }
        if (order_request_inner.trader_id != TraderId::Price_Enforcer) {
            accounts_data
                .index_ref(order_request_inner.trader_id)
                .lock()
                .unwrap()
                .net_cents_balance -= order_request_inner.price * order_request_inner.amount;
        }
    };
    if (order_request_inner.order_type == crate::orderbook::OrderType::Sell) {
        // ISSUE: This should decrement cents_balance to avoid racing to place two orders before updating cents_balance
        // check if current cash balance - outstanding orders supports order
        // nevermind, as long as I acquire and hold a lock during the entire order placement attempt, it should be safe
        if ((*accounts_data
            .index_ref(order_request_inner.trader_id)
            .lock()
            .unwrap()
            .net_asset_balances
            .index_ref(symbol)
            .lock()
            .unwrap()
            //+ 1000 allow 1000 shares
            < <usize as TryInto<i64>>::try_into(order_request_inner.amount).unwrap())
            && order_request_inner.trader_id != TraderId::Price_Enforcer)
        {
            println!("Error: attempted short sell");
            return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage{
                side: order_request_inner.order_type,
                price: order_request_inner.price,
                symbol: order_request_inner.symbol,
                error_details: "Error Placing Order: The total amount of this trade would take your account over 1000 shares short"
            });
        }
        if (order_request_inner.trader_id != TraderId::Price_Enforcer) {
            *accounts_data
                .index_ref(order_request_inner.trader_id)
                .lock()
                .unwrap()
                .net_asset_balances
                .index_ref(symbol)
                .lock()
                .unwrap() -= <usize as TryInto<i64>>::try_into(order_request_inner.amount).unwrap();
        }
    };

    println!(
        "Account has {:?} lots of {:?}",
        &accounts_data
            .index_ref(order_request_inner.trader_id)
            .lock()
            .unwrap()
            .asset_balances
            .index_ref(symbol)
            .lock()
            .unwrap(),
        symbol
    );
    println!(
        "Account has {:?} cents",
        &accounts_data
            .index_ref(order_request_inner.trader_id)
            .lock()
            .unwrap()
            .cents_balance
    );

    // let orderbook = data.index_ref(symbol);
    // let jnj_orderbook = data.index_ref(&crate::macro_calls::TickerSymbol::JNJ);
    // jnj_orderbook.lock().unwrap().print_book_state();
    // ISSUE: need to borrow accounts as mutable without knowing which ones will be needed to be borrowed
    // maybe pass in immutable reference to entire account state, and only acquire the locks for the mutex's that it turns out we need

    // Server Generated Order ID
    let order_id = order_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let order = data
        .index_ref(&symbol.clone())
        .lock()
        .unwrap()
        .handle_incoming_order_request(
            order_request_inner.clone(),
            accounts_data,
            relay_server_addr,
            order_counter,
            order_id,
            start_time
        );

    // very gross, should deal with
    match order {
        Ok(inner) => {
            return OrderPlaceResponse::OrderConfirmMessage(OrderConfirmMessage {
                order_info: inner,
            })
        }
        Err(err) => {
            return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage {
                side: order_request_inner.order_type,
                price: order_request_inner.price,
                symbol: order_request_inner.symbol,
                error_details: "unknown error when placing order",
            })
        }
    }
}

pub fn cancel_order<'a>(
    cancel_request: CancelRequest,
    data: &crate::config::GlobalOrderBookState,
    accounts_data: &crate::config::GlobalAccountState,
    relay_server_addr: &web::Data<Addr<crate::connection_server::Server>>,
    order_counter: &web::Data<Arc<AtomicUsize>>,
) -> crate::api_messages::OrderCancelResponse<'a> {
    let cancel_request_inner = cancel_request;
    let symbol = &cancel_request_inner.symbol;
    let order = data
        .index_ref(symbol)
        .lock()
        .unwrap()
        .handle_incoming_cancel_request(
            cancel_request_inner,
            order_counter,
            relay_server_addr,
            accounts_data,
        );
    // todo: add proper error handling/messaging
    // instead of returning none, this should return Result and I can catch it here to propagate up actix framework
    match order {
        Ok(inner) => {
            // handles freeing up credit limits used by order
            match inner.order_type {
                OrderType::Buy => {
                    // increase available funds
                    accounts_data
                        .index_ref(inner.trader_id)
                        .lock()
                        .unwrap()
                        .net_cents_balance += inner.amount * inner.price;
                }
                OrderType::Sell => {
                    // increase available assets
                    // need to dereference because each asset balance is a separate mutex (should this be changed?)
                    *accounts_data
                        .index_ref(inner.trader_id)
                        .lock()
                        .unwrap()
                        .net_asset_balances
                        .index_ref(&inner.symbol)
                        .lock()
                        .unwrap() +=  <usize as TryInto<i64>>::try_into(inner.amount).unwrap()
                }
            }
            return crate::api_messages::OrderCancelResponse::CancelConfirmMessage(
                CancelConfirmMessage { order_info: inner },
            );
        }
        Err(err) => {
            return crate::api_messages::OrderCancelResponse::CancelErrorMessage(
                //to-do
                CancelErrorMessage {
                    side: OrderType::Sell,
                    price: cancel_request_inner.price,
                    symbol: cancel_request_inner.symbol,
                    error_details: "unknown error when placing order",
                    order_id: cancel_request_inner.order_id,
                },
            )
        }
    }
}

pub struct MyWebSocketActor {
    connection_ip: TraderIp,
    associated_id: TraderId,
    hb: Instant,
    global_state: web::Data<GlobalState>,
    // global_account_state: crate::config::GlobalAccountState,
    // global_orderbook_state: crate::config::GlobalOrderBookState,
    // for testing.
    start_time: web::Data<SystemTime>,
    t_orders: usize,
    relay_server_addr: web::Data<Addr<crate::connection_server::Server>>,
    order_counter: web::Data<Arc<AtomicUsize>>,
}

impl MyWebSocketActor {
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                warn!("Client Timed Out :(");
                ctx.stop();
                return;
            }
            //println!("sent ping message");
            ctx.ping(b"");
        });
    }
}

// Add this near your other handler functions
pub async fn start_game(global_state: web::Data<GlobalState>) -> Result<HttpResponse, Error> {
    {
        let mut game_started = global_state.game_started.lock().unwrap();
        if *game_started {
            return Ok(HttpResponse::BadRequest().body("Game has already started."));
        }
        *game_started = true;
    }

    // Notify all connected clients that the game has started
    Broker::<SystemBroker>::issue_async(GameStartedMessage("GameStarted".to_string()));

    Ok(HttpResponse::Ok().body("Game started successfully."))
}

pub async fn websocket(
    req: HttpRequest,
    stream: web::Payload,
    // orderbook_data: crate::config::GlobalOrderBookState,
    // accounts_data: crate::config::GlobalAccountState,
    state_data: web::Data<GlobalState>,
    start_time: web::Data<SystemTime>,
    relay_server_addr: web::Data<Addr<crate::connection_server::Server>>,
    order_counter: web::Data<Arc<AtomicUsize>>,
) -> Result<HttpResponse, Error> {
    let conninfo = req.connection_info().clone();
    
    println!("{:?}", req.headers());

    info!(
        "New websocket connection with peer_addr: {:?}, id: {:?}",
        conninfo.peer_addr(),
        req.headers()
            .get("Sec-WebSocket-Protocol")
            .unwrap()
            .to_str()
            .unwrap()
    );

    let protocol = req.headers()
        .get("Sec-WebSocket-Protocol")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            error!("Missing Sec-WebSocket-Protocol header");
            Error::from(actix_web::error::ErrorBadRequest("Missing credentials"))
        })?;

    let creds: Vec<&str> = protocol.split('|').collect();
    if creds.len() != 2 {
        error!("Invalid credential format");
        return Ok(HttpResponse::BadRequest().body("Invalid credential format"));
    }

    let username = creds[0];
    let password = creds[1];

    // Parse username into TraderId
    let trader_id = match <TraderId as std::str::FromStr>::from_str(username) {
        Ok(id) => id,
        Err(_) => {
            error!("Invalid trader ID: {}", username);
            return Ok(HttpResponse::BadRequest().body("Invalid trader ID"));
        }
    };

    // Validate credentials
    let stored_password = state_data
        .global_account_state
        .index_ref(trader_id)
        .lock()
        .unwrap()
        .password;

    let password_chars: Vec<char> = password.chars().collect();
    if password_chars.len() != 4 {
        error!("Invalid password length for trader ID: {}. Expected 4 characters, got {}", 
            username, password_chars.len());
        return Ok(HttpResponse::BadRequest()
            .body(format!("Invalid password length. Expected 4 characters, got {}", password_chars.len())));
    }

    if stored_password.to_vec() != password_chars {
        error!("Invalid password for trader ID: {}", username);
        return Ok(HttpResponse::Unauthorized().body("Invalid credentials"));
    }

    ws::start_with_protocols(
    MyWebSocketActor {
        connection_ip: req
            .connection_info()
            .realip_remote_addr()
            .unwrap()
            .parse()
            .unwrap(),
        associated_id: trader_id,
        hb: Instant::now(),
        global_state: state_data.clone(),
        start_time: start_time.clone(),
        t_orders: 0,
        relay_server_addr: relay_server_addr.clone(),
        order_counter: order_counter.clone(),
    },
    &[protocol],
    &req,
    stream,
    )
/* 
    ws::start(
        MyWebSocketActor {
            connection_ip: req
                .connection_info()
                .realip_remote_addr()
                .unwrap()
                .parse()
                .unwrap(),
            associated_id: <TraderId as std::str::FromStr>::from_str(
                req.headers()
                    .get("Sec-WebSocket-Protocol")
                    .unwrap()
                    .to_str()
                    .unwrap(),
            )
            .unwrap(),
            hb: Instant::now(),
            global_state: state_data.clone(),
            // global_account_state: accounts_data.clone(),
            // global_orderbook_state: orderbook_data.clone(),
            start_time: start_time.clone(),
            t_orders: 0,
            relay_server_addr: relay_server_addr.clone(),
            order_counter: order_counter.clone(),
        },
        &req,
        stream,
    )
*/
}

impl Actor for MyWebSocketActor {
    type Context = ws::WebsocketContext<Self>;

    // Start the heartbeat process for this connection
    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_system_async::<orderbook::OrderBook>(ctx);
        self.subscribe_system_async::<GameStartedMessage>(ctx);
        // self.subscribe_system_async::<orderbook::LimLevUpdate>(ctx);
        self.relay_server_addr.do_send(OpenMessage {
            ip: self.connection_ip,
            addr: ctx.address().recipient(),
        });
        println!("Subscribed");
        self.hb(ctx);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let account_id = self.associated_id;
        let curr_actor = &mut self
            .global_state
            .global_account_state
            .index_ref(account_id)
            .lock()
            .unwrap()
            .current_actor;
        self.relay_server_addr.do_send(CloseMessage {
            ip: self.connection_ip,
            addr: ctx.address().recipient(),
        });

        match curr_actor {
            Some(x) => {
                *curr_actor = None;
            }
            None => warn!("curr_actor already None"),
        }
        info!(
            "Websocket connection ended (peer_ip:{}).",
            self.connection_ip
        );
        info!(
            "curr_order_count {:?}",
            self.order_counter
                .load(std::sync::atomic::Ordering::Relaxed)
        )
    }
}

/// Define handler for `Fill` message, triggers when one of your orders is involved in a trade
impl Handler<Arc<crate::api_messages::OrderFillMessage>> for MyWebSocketActor {
    type Result = ();

    fn handle(&mut self, msg: Arc<crate::api_messages::OrderFillMessage>, ctx: &mut Self::Context) {
        // let fill_event = msg;
        let hack_msg = api_messages::OutgoingMessage::OrderFillMessage(*msg);

        ctx.text(serde_json::to_string(&hack_msg).unwrap());
    }
}

/// Define handler for `OrderBookUpdate` message
impl Handler<orderbook::OrderBook> for MyWebSocketActor {
    type Result = ();

    fn handle(&mut self, msg: orderbook::OrderBook, ctx: &mut Self::Context) {
        println!("Orderbook Message Received");
        // msg.print_book_state();
        ctx.text(format!("{:?}", &msg.get_book_state()));
    }
}

// TODO: generalize to Handler<Arc<T>> for generic message types
// Implement a marker trait (something like LOBChangeMessage)
// Handling these market data event messages is just sending out json'd version of the message struct
// Key word: Blanket implementations
impl Handler<Arc<OutgoingMessage>> for MyWebSocketActor {
    type Result = ();
    fn handle(&mut self, msg: Arc<OutgoingMessage>, ctx: &mut Self::Context) {
        // there has to be a nicer way to do this, but cant figure out how to access inner type when doing a default match
        // these messages are sent by Server detailed in connection_server.rs
        ctx.text(serde_json::to_string(&*msg).unwrap());
        // match *msg {
        //     OutgoingMessage::NewRestingOrderMessage(m) => {
        //         println!("NewRestingOrderMessage Received");
        //         ctx.text(serde_json::to_string(&msg).unwrap());
        //     }
        //     OutgoingMessage::TradeOccurredMessage(m) =>  {
        //         println!("TradeOccurredMessage Received");
        //         ctx.text(serde_json::to_string(&m).unwrap());
        //     }
        //     OutgoingMessage::CancelOccurredMessage(m) => {
        //         println!("CancelOccurredMessage Received");
        //         ctx.text(serde_json::to_string(&m).unwrap());
        //     },
        // }
    }
}

// impl Handler<Arc<orderbook::LimLevUpdate>> for MyWebSocketActor {
//     type Result = ();

//     fn handle(&mut self, msg: Arc<orderbook::LimLevUpdate>, ctx: &mut Self::Context) {
//         // println!("LimLevUpdate Message Received");
//         // msg.print_book_state()
//         ctx.text(serde_json::to_string(&(*msg).clone()).unwrap());
//     }
// }

// The `StreamHandler` trait is used to handle the messages that are sent over the socket.
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocketActor {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let game_started = *self.global_state.game_started.lock().unwrap();

        // Handle messages as usual if the game has started
        match msg {
            Ok(ws::Message::Text(text)) => {
                if !game_started {
                    // If the game hasn't started, reject actions but allow connections
                    ctx.text(format!("{{MiscError : Game has not started yet.}}"));
                    return;
                }

                let t_start = SystemTime::now();
                self.t_orders += 1;
                
                // handle incoming JSON
                info!("{}", &text.to_string());
                
                let incoming_message = match serde_json::from_str::<IncomingMessage>(&text.to_string()) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Failed to parse incoming message: {}", e);
                        ctx.text(format!("{{\"error\": \"Invalid message format: {}\"}}", e));
                        return;
                    }
                };

                let connection_ip = self.connection_ip;

                match incoming_message {
                    IncomingMessage::OrderRequest(order_req) => {
                        let password_needed = self
                            .global_state
                            .global_account_state
                            .index_ref(order_req.trader_id)
                            .lock()
                            .unwrap()
                            .password;
                        if (password_needed != order_req.password) {
                            // Should return a standardized error message for the client instead of text
                            warn!("Invalid password for provided trader_id: {}", connection_ip);
                            // This should be a proper error
                            ctx.text("invalid password for provided trader id.");
                        } else {
                            let res = add_order(
                                order_req,
                                &self.global_state.global_orderbook_state,
                                &self.global_state.global_account_state,
                                &self.relay_server_addr,
                                &self.order_counter,
                                &self.start_time
                            );
                            // elapsed is taking a non negligible time
                            let secs_elapsed = self
                                .start_time
                                .clone()
                                .into_inner()
                                .as_ref()
                                .elapsed()
                                .unwrap();
                            println!(
                                "time_elapsed from start: {:?}",
                                usize::try_from(secs_elapsed.as_secs()).unwrap()
                            );
                            println!(
                                "total orders processed:{:?}",
                                self.order_counter.load(std::sync::atomic::Ordering::SeqCst)
                            );
                            println!(
                                "orders/sec: {:?}",
                                self.order_counter.load(std::sync::atomic::Ordering::SeqCst)
                                    / usize::try_from(secs_elapsed.as_secs()).unwrap()
                            );

                            // println!("res: {}", res);
                            // let msg = self.global_state.global_orderbook_state.index_ref(&t.symbol).lock().unwrap().to_owned();
                            // println!("Issuing Async Msg");
                            // Broker::<SystemBroker>::issue_async(msg);
                            // println!("Issued Async Msg");
                            // println!("{:?}", serde_json::to_string_pretty(&t));

                            // measured @~14microseconds.
                            // for some reason goes up as more orders are added :(
                            match &res {
                                OrderPlaceResponse::OrderPlaceErrorMessage(msg) => {
                                    ctx.text(serde_json::to_string(&res).unwrap());
                                }
                                OrderPlaceResponse::OrderConfirmMessage(msg) => {
                                    // required for logging/state recovery in case of crashes
                                    info!(
                                        "ORDER DUMP: {}",
                                        serde_json::to_string(&order_req).unwrap()
                                    );

                                    ctx.text(serde_json::to_string(&res).unwrap());
                                }
                            }
                        }
                    }
                    IncomingMessage::CancelRequest(cancel_req) => {
                        let password_needed = self
                            .global_state
                            .global_account_state
                            .index_ref(cancel_req.trader_id)
                            .lock()
                            .unwrap()
                            .password;
                        if (password_needed != cancel_req.password) {
                            warn!("Invalid password for provided trader_id: {}", connection_ip);
                            // This should be a proper error
                            ctx.text("invalid password for provided trader id.");
                        } else {
                            let res = cancel_order(
                                cancel_req,
                                &self.global_state.global_orderbook_state,
                                &self.global_state.global_account_state,
                                &self.relay_server_addr,
                                &self.order_counter,
                            );
                            // elapsed is taking a non negligible time
                            let secs_elapsed = self
                                .start_time
                                .clone()
                                .into_inner()
                                .as_ref()
                                .elapsed()
                                .unwrap();
                            println!(
                                "time_elapsed from start: {:?}",
                                usize::try_from(secs_elapsed.as_secs()).unwrap()
                            );
                            println!(
                                "total orders processed:{:?}",
                                self.order_counter.load(std::sync::atomic::Ordering::SeqCst)
                            );
                            println!(
                                "orders/sec: {:?}",
                                self.order_counter.load(std::sync::atomic::Ordering::SeqCst)
                                    / usize::try_from(secs_elapsed.as_secs()).unwrap()
                            );
                            // need to match onto cancel response possibilities

                            match &res {
                                crate::api_messages::OrderCancelResponse::CancelConfirmMessage(
                                    msg,
                                ) => {
                                    // required for logging/state recovery in case of crashes
                                    info!(
                                        "CANCEL DUMP: {}",
                                        serde_json::to_string(&cancel_req).unwrap()
                                    );

                                    ctx.text(serde_json::to_string(&res).unwrap());
                                }
                                crate::api_messages::OrderCancelResponse::CancelErrorMessage(
                                    msg,
                                ) => {
                                    ctx.text(serde_json::to_string(&res).unwrap());
                                }
                            }
                        };
                    }
                    IncomingMessage::AccountInfoRequest(account_info_request) => {
                        println!("Received AccountInfoRequest");
                        let password_needed = self
                            .global_state
                            .global_account_state
                            .index_ref(account_info_request.trader_id)
                            .lock()
                            .unwrap()
                            .password;
                        if (password_needed != account_info_request.password) {
                            warn!("Invalid password for provided trader_id: {}", connection_ip);
                            // This should be a proper error
                            ctx.text("invalid password for provided trader id.");
                        } else {
                            // should basically just send back serialized TraderAccount
                            // need to attach all active order objects to TraderAccount in orderbook
                            let account = self
                                .global_state
                                .global_account_state
                                .index_ref(account_info_request.trader_id)
                                .lock()
                                .unwrap();
                            // &* feels gross, not sure if there is a nicer/more performant solution
                            ctx.text(format!("{{\"AccountInfo\" : {}}}",serde_json::to_string(&*account).unwrap()))
                        }
                    }
                    IncomingMessage::GameStateRequest => {ctx.text(format!("{{\"GameState\" : {}}}",serde_json::to_string(&self.global_state.global_orderbook_state).unwrap()));}
                }
            }

            // Ping/Pong will be used to make sure the connection is still alive
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                //info!("Ping Received");
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                //info!("Pong Received");
                self.hb = Instant::now();
            }
            // Text will echo any text received back to the client (for now)
            // Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Close(reason)) => {
                let account_id = self.associated_id;
                self.global_state
                    .global_account_state
                    .index_ref(account_id)
                    .lock()
                    .unwrap()
                    .current_actor = None;
                info!("Received close message, closing context.");
                ctx.close(reason);
                ctx.stop();
            }
            _ => {
                error!("Error reading message, stopping context.");
                // should send generic error message to client as well
                ctx.stop();
            }
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        ctx.stop()
    }

    fn started(&mut self, ctx: &mut Self::Context) {
        // Broker::<SystemBroker>::issue_async(self.global_state.global_orderbook_state);
        let connection_ip = self.connection_ip;
        let account_id = self.associated_id;

        println!("Trader with id {:?} connected.", account_id);
        {
            let curr_actor = &mut self
                .global_state
                .global_account_state
                .index_ref(account_id)
                .lock()
                .unwrap()
                .current_actor;

            match curr_actor {
                Some(x) => {
                    if (connection_ip
                        != env::var("GRAFANAIP")
                            .expect("$GRAFANAIP is not set")
                            .parse::<TraderIp>()
                            .unwrap())
                    {
                        error!("Trader_id already has websocket connected");
                        ctx.stop();
                    }
                }
                None => *curr_actor = Some(ctx.address()),
            }
        }

        let mut account = self
            .global_state
            .global_account_state
            .index_ref(account_id)
            .lock()
            .unwrap();
        // &* feels gross, not sure if there is a nicer/more performant solution
        ctx.text(format!("{{\"AccountInfo\" : {}}}",serde_json::to_string(&*account).unwrap()));
        ctx.text(format!("{{\"GameState\" : {}}}",serde_json::to_string(&self.global_state.global_orderbook_state).unwrap()));
        let game_started = *self.global_state.game_started.lock().unwrap();
        if game_started {
            ctx.text(format!("{{\"GameStartedMessage\" : \"GameStarted\"}}"));
        }
        let message_queue = &mut account.message_backup;
        // Message queue should support OrderFillMessage, not TradeOccurredMessage or Fill
        // to inform client side account state sync

        while (message_queue.size() != 0) {
            let order_fill_msg = message_queue.remove().unwrap();
            let hack_msg = api_messages::OutgoingMessage::OrderFillMessage(*order_fill_msg);
            ctx.text(serde_json::to_string(&hack_msg).unwrap());
        }
        
    }

}


impl Handler<GameStartedMessage> for MyWebSocketActor {
    type Result = ();

    fn handle(&mut self, msg: GameStartedMessage, ctx: &mut Self::Context) {
        if msg.0 == "GameStarted" {
            ctx.text(format!("{{\"GameStartedMessage\" : \"GameStarted\"}}"));
        }
    }
}


// Add this new handler
impl Handler<GameEndMessage> for MyWebSocketActor {
    type Result = ();

    fn handle(&mut self, msg: GameEndMessage, ctx: &mut Self::Context) {
        ctx.text(format!("{{\"GameEndMessage\" : {}}}",serde_json::to_string(&msg.final_score).unwrap()));
    }
}


pub async fn end_game(global_state: web::Data<GlobalState>) -> Result<HttpResponse, Error> {
    {
        let mut game_started = global_state.game_started.lock().unwrap();
        if !*game_started {
            return Ok(HttpResponse::BadRequest().body("Game has not started yet."));
        }
        *game_started = false;
    }

    // Collect and sort final balances
    let accounts = &global_state.global_account_state;
    let mut final_standings: Vec<(TraderId, usize)> = Vec::new();
    
    // Collect balances for each trader
    for trader_id in config::TraderId::iter() {
        if trader_id != TraderId::Price_Enforcer {
            let account = accounts.index_ref(trader_id).lock().unwrap();
            final_standings.push((trader_id, account.cents_balance));
        }
    }

    // Sort by balance in descending order
    final_standings.sort_by(|a, b| b.1.cmp(&a.1));
    
    // Print final standings
    println!("Final Standings:");
    for (rank, (trader_id, balance)) in final_standings.iter().enumerate() {
        println!("Rank {}: Trader {:?} - Balance: {} cents", rank + 1, trader_id, balance);
    }

    for (trader_id, balance) in final_standings.iter() {
        let end_message = GameEndMessage {
            final_score: *balance,
        };

        if let Some(actor) = &accounts.index_ref(*trader_id).lock().unwrap().current_actor {
            actor.do_send(end_message);
        }
    }

    Ok(HttpResponse::Ok().json(final_standings))
}