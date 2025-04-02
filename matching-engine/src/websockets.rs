use actix::prelude::*;
use actix_web::Error;
use actix_web_actors::ws;
use log::info;
use std::env;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use actix_broker::BrokerSubscribe;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(4);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
use crate::api_messages::{
    self, CancelConfirmMessage, CancelErrorMessage, CancelRequest, IncomingMessage,
    OrderConfirmMessage, OrderPlaceErrorMessage, OrderPlaceResponse,
    OrderRequest, OutgoingMessage
};
use crate::message_types::{CloseMessage, GameStartedMessage, OpenMessage};
use crate::orderbook::TraderId;
use actix_web::{web, HttpRequest, HttpResponse};
extern crate env_logger;

use crate::config::TraderIp;
pub use crate::orderbook::OrderType;
use crate::GlobalState;


pub fn add_order<'a>(
    order_request: OrderRequest,
    data: &crate::config::GlobalOrderBookState,
    accounts_data: &crate::config::GlobalAccountState,
    relay_server_addr: &web::Data<Addr<crate::connection_server::Server>>,
    order_counter: &web::Data<Arc<AtomicUsize>>,
    start_time: &web::Data<SystemTime>
) -> OrderPlaceResponse<'a> {

    let order_request_inner = order_request;
    let symbol = &order_request_inner.symbol;
    
    // Check active orders capacity
    let trader_account = accounts_data
    .index_ref(order_request_inner.trader_id)
    .lock()
    .unwrap();

    if trader_account.active_orders.len() >= trader_account.active_orders.capacity() {
        return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage {
            side: order_request_inner.order_type,
            price: order_request_inner.price,
            symbol: order_request_inner.symbol,
            error_details: "Trader has reached maximum number of active orders"
        });
    }

    drop(trader_account);

   // Check price level bounds
   let orderbook = data.index_ref(&symbol).lock().unwrap();
   let max_price = orderbook.buy_side_limit_levels.len();
   if order_request_inner.price >= max_price {
       return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage {
           side: order_request_inner.order_type,
           price: order_request_inner.price,
           symbol: order_request_inner.symbol,
           error_details: "Price exceeds maximum allowed price"
       });
   }

   if order_request_inner.amount > 10_000 {
    return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage {
        side: order_request_inner.order_type,
        price: order_request_inner.price,
        symbol: order_request_inner.symbol,
        error_details: "Volume exceeds maximum allowed single-order volume"
    });
   }

   // Check limit level capacity based on order type
   let level_orders = match order_request_inner.order_type {
       OrderType::Buy => &orderbook.buy_side_limit_levels[order_request_inner.price].orders,
       OrderType::Sell => &orderbook.sell_side_limit_levels[order_request_inner.price].orders,
   };

   // Compare against vector capacity
   if level_orders.len() >= level_orders.capacity() {
       return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage {
           side: order_request_inner.order_type,
           price: order_request_inner.price,
           symbol: order_request_inner.symbol,
           error_details: "Price level is at capacity"
       });
   }

   // Drop the orderbook lock before proceeding with existing logic
   drop(orderbook);


    // Todo: refactor into match statement, put into actix guard?
    if order_request_inner.order_type == crate::orderbook::OrderType::Buy {
        let cent_value = &order_request_inner.amount * &order_request_inner.price;
        if (accounts_data
            .index_ref(order_request_inner.trader_id)
            .lock()
            .unwrap()
            .net_cents_balance
            < cent_value)
            && order_request_inner.trader_id != TraderId::Price_Enforcer
        {
            return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage{
                side: order_request_inner.order_type,
                price: order_request_inner.price,
                symbol: order_request_inner.symbol,
                error_details: "Error Placing Order: The total value of order is greater than current account balance"
            });
        }
        if order_request_inner.trader_id != TraderId::Price_Enforcer {
            accounts_data
                .index_ref(order_request_inner.trader_id)
                .lock()
                .unwrap()
                .net_cents_balance -= order_request_inner.price * order_request_inner.amount;
        }
    };
    if order_request_inner.order_type == crate::orderbook::OrderType::Sell {
        if (*accounts_data
            .index_ref(order_request_inner.trader_id)
            .lock()
            .unwrap()
            .net_asset_balances
            .index_ref(symbol)
            .lock()
            .unwrap()
            < <usize as TryInto<i64>>::try_into(order_request_inner.amount).unwrap())
            && order_request_inner.trader_id != TraderId::Price_Enforcer
        {
            return OrderPlaceResponse::OrderPlaceErrorMessage(OrderPlaceErrorMessage{
                side: order_request_inner.order_type,
                price: order_request_inner.price,
                symbol: order_request_inner.symbol,
                error_details: "Error Placing Order: The total amount of this trade would take your account short"
            });
        }
        if order_request_inner.trader_id != TraderId::Price_Enforcer {
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
        Err(_err) => {
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
    start_time: web::Data<SystemTime>,
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
            ctx.ping(b"");
        });
    }
}

pub async fn websocket(
    req: HttpRequest,
    stream: web::Payload,
    state_data: web::Data<GlobalState>,
    start_time: web::Data<SystemTime>,
    relay_server_addr: web::Data<Addr<crate::connection_server::Server>>,
    order_counter: web::Data<Arc<AtomicUsize>>,
) -> Result<HttpResponse, Error> {
    let conninfo = req.connection_info().clone();

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

    ws::start(
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
        relay_server_addr: relay_server_addr.clone(),
        order_counter: order_counter.clone(),
    },
    &req,
    stream,
    )
}

impl Actor for MyWebSocketActor {
    type Context = ws::WebsocketContext<Self>;

    // Start the heartbeat process for this connection
    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_system_async::<GameStartedMessage>(ctx);
        self.relay_server_addr.do_send(OpenMessage {
            ip: self.connection_ip,
            addr: ctx.address().recipient(),
        });
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
    }
}

// The `StreamHandler` trait is used to handle the messages that are sent over the socket.
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocketActor {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {

        // Handle messages as usual if the game has started
        match msg {
            Ok(ws::Message::Text(text)) => {
                if !*self.global_state.game_started.lock().unwrap() {
                    // If the game hasn't started, reject actions but allow connections
                    ctx.text("{{\"Error\" : \"Game has not started yet.\"}}");
                    return;
                }
                
                let incoming_message = match serde_json::from_str::<IncomingMessage>(&text.to_string()) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Failed to parse incoming message: {}", e);
                        ctx.text(format!("{{\"Error\": \"Invalid message format: {}\"}}", e));
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
                        if password_needed != order_req.password {
                            warn!("Invalid password for provided trader_id: {}", connection_ip);

                            ctx.text("{{\"Error\" : \"invalid password for provided trader id.\"}}");
                        } else {
                            let res = add_order(
                                order_req,
                                &self.global_state.global_orderbook_state,
                                &self.global_state.global_account_state,
                                &self.relay_server_addr,
                                &self.order_counter,
                                &self.start_time
                            );

                            match &res {
                                OrderPlaceResponse::OrderPlaceErrorMessage(msg) => {
                                    ctx.text(serde_json::to_string(&res).unwrap());
                                }
                                OrderPlaceResponse::OrderConfirmMessage(msg) => {

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
                            ctx.text("{{\"Error\" : \"invalid password for provided trader id.\"}}");
                        } else {
                            let res = cancel_order(
                                cancel_req,
                                &self.global_state.global_orderbook_state,
                                &self.global_state.global_account_state,
                                &self.relay_server_addr,
                                &self.order_counter,
                            );

                            ctx.text(serde_json::to_string(&res).unwrap());
                        };
                    }

                    IncomingMessage::AccountInfoRequest(account_info_request) => {
                        let password_needed = self
                            .global_state
                            .global_account_state
                            .index_ref(account_info_request.trader_id)
                            .lock()
                            .unwrap()
                            .password;
                        if (password_needed != account_info_request.password) {
                            warn!("Invalid password for provided trader_id: {}", connection_ip);
                            ctx.text("{{\"Error\" : \"invalid password for provided trader id.\"}}");
                        } else {
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
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
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
        
    }

}