use actix_broker::Broker;
use actix_broker::SystemBroker;
use actix_web::HttpRequest;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Error};
use actix_cors::Cors;
use api_messages::OrderRequest;
use api_messages::OutgoingMessage;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use message_types::GameStartedMessage;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use websockets::websocket;
extern crate env_logger;
use crate::config::TraderId;
use crate::websockets::add_order;
use actix::Actor;
use std::net::Ipv4Addr;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod accounts;
mod api_messages;
mod config;
mod connection_server;
mod message_types;
mod orderbook;
mod websockets;
// mod parser;

use std::sync::atomic::AtomicUsize;

use std::time::SystemTime;

use std::sync::Arc;

pub use crate::accounts::TraderAccount;
// pub use crate::orderbook::TickerSymbol;
pub use crate::accounts::quickstart_trader_account;
pub use crate::orderbook::quickstart_order_book;
pub use crate::orderbook::OrderBook;
pub use crate::orderbook::OrderType;

use config::AssetBalances;
use config::TickerSymbol;

use config::GlobalAccountState;
use config::GlobalOrderBookState;

use rev_lines::RevLines;



#[derive(Debug, Serialize, Deserialize)]
struct GlobalState {
    global_orderbook_state: GlobalOrderBookState,
    global_account_state: GlobalAccountState,
    #[serde(skip)]
    game_started: Arc<Mutex<bool>>, // Shared flag to track game state
}

impl GlobalState {
    fn dump_state(self) {
        info!("{:?}", json!(self))
    }
}

fn load_state(
    log_file: fs::File,
    order_counter: &web::Data<Arc<AtomicUsize>>,
    relay_server_addr: &web::Data<actix::Addr<crate::connection_server::Server>>,
    start_time: &web::Data<SystemTime>
) -> Option<GlobalState> {
    // todo: convert to Result<> instead of Option<>
    // search from bottom up until we find a state dump, take that as ground truth
    let rev_lines = RevLines::new(log_file);
    let enumerated_lines = rev_lines.enumerate();
    let mut successful_orders_and_cancels: Vec<api_messages::IncomingMessage> = Vec::new();
    for (i, line) in enumerated_lines {
        let line_u = line.unwrap();
        let len = &line_u.len();
        
        if len > &50 {
            if &line_u[0..45] == r#" INFO  main                    > STATE DUMP: "# {
                info!("Found state dump!");
                info!("state: {:?}", &line_u[45..]);
                let gs: GlobalState = serde_json::from_str(&line_u[45..]).unwrap();
                // let mut res;
                // get line number of last state dump
                info!("line of last dump: {:?}", i);
                // info!("{:?}", successful_orders_and_cancels);
                // info!("{:?}", enumerated_lines.rev().nth(0));
                // we are now at the last state dump, and should reverse (i.e. read forwards) until the end of file
                // iterate over all successful orders/cancels since last state dump, calling on add_order() or cancel_order()
                for incoming_message in successful_orders_and_cancels.iter().rev() {
                    // handle incoming message as if it was live to update state
                    // can ignore some checks as all logged messages were successful during initial run
                    // i.e. there should be no errors.
                    match *incoming_message {
                        api_messages::IncomingMessage::OrderRequest(order_request) => {
                            info!("Order request found");
                            _ = add_order(
                                order_request,
                                &gs.global_orderbook_state,
                                &gs.global_account_state,
                                relay_server_addr,
                                order_counter,
                                start_time
                            );
                        }
                        api_messages::IncomingMessage::CancelRequest(cancel_request) => {
                            info!("Cancel request found");
                            _ = websockets::cancel_order(
                                cancel_request,
                                &gs.global_orderbook_state,
                                &gs.global_account_state,
                                relay_server_addr,
                                order_counter,
                            );
                        }
                        api_messages::IncomingMessage::AccountInfoRequest(account_info_request) => {
                            info!("Account info request found")
                        }
                        _ => ()
                        
                    }
                }
                // return the final reconstructed global state
                return Some(gs);

            // If we are still searching for the last state dump, parse all successful order and cancels
            } else if &line_u[0..45]
                == r#" INFO  main::websockets        > ORDER DUMP: "#
            {
                info!("Order request line found");
                let order_req: api_messages::OrderRequest =
                    serde_json::from_str(&line_u[45..]).unwrap();
                successful_orders_and_cancels
                    .push(api_messages::IncomingMessage::OrderRequest(order_req));
            } else if &line_u[0..46]
                == r#" INFO  main::websockets        > CANCEL DUMP: "#
            {
                let cancel_req: api_messages::CancelRequest =
                    serde_json::from_str(&line_u[46..]).unwrap();
                successful_orders_and_cancels
                    .push(api_messages::IncomingMessage::CancelRequest(cancel_req));
            }
        }
    }
    None
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let start_time = web::Data::new(SystemTime::now());
    let order_counter = web::Data::new(Arc::new(AtomicUsize::new(0)));
    let relay_server = web::Data::new(connection_server::Server::new().start());
    let global_state = web::Data::new(GlobalState {
        global_orderbook_state: config::GlobalOrderBookState::new(),
        global_account_state: config::GlobalAccountState::new(),
        game_started: Arc::new(Mutex::new(false)),
    });

    pretty_env_logger::init();
    info!("Starting...");
    println!("Starting matching engine server...");

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::default().allow_any_origin().allow_any_method().allow_any_header())
            .app_data(global_state.clone())
            .app_data(start_time.clone())
            .app_data(relay_server.clone())
            .app_data(order_counter.clone())
            .route("/start_game", web::get().to(websockets::start_game))
            .route("/end_game", web::get().to(websockets::end_game))
            .service(
                web::scope("/orders")
                    .route("/ws", web::get().to(websockets::websocket)),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
