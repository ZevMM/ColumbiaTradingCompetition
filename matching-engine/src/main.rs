use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use api_messages::OrderRequest;
use api_messages::OutgoingMessage;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
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
    pretty_env_logger::init();
    info!("Starting...");

    // todo: convert to proper strongly typed deserialization
    // convert asset enum to be like AAPL(Asset)
    // Asset as struct which includes long name, max price, etc.
    // convert global state to an enum generated at compile time
    // let file = File::open(Path::new("./config.json")).unwrap();
    // let reader = BufReader::new(file);

    // let value: Value = serde_json::from_reader(reader).unwrap();

    // let assets_arr = value["assets"].as_array().unwrap();
    // let accounts_arr = value["accounts"].as_array().unwrap();

    // should start main server actor here, and pass in as cloned app data to websocket endpoint
    let relay_server = connection_server::Server::new().start();

    let order_count = Arc::new(AtomicUsize::new(0));

    // to do: add actix guards to confirm correctly formed requests etc.
    // to do: add actix guards to confirm credit checks etc.
    // to do: move this declaration to macro_calls file to generate fields automatically
    let global_orderbook_state = config::GlobalOrderBookState::new();
 
    // todo: abstract to config file, this is disgusting (see config.json -> build.rs -> config.rs)
    let global_account_state = config::GlobalAccountState::new();

    // todo: abstract to config file, this is disgusting (see config.json -> build.rs -> config.rs)
    /* 
    *global_account_state
        .Columbia_A
        .lock()
        .unwrap()
        .asset_balances
        .index_ref(&config::TickerSymbol::TT)
        .lock()
        .unwrap() = 30000;
    *global_account_state
        .Columbia_A
        .lock()
        .unwrap()
        .asset_balances
        .index_ref(&config::TickerSymbol::TS)
        .lock()
        .unwrap() = 30000;
    */
    // *global_account_state
    //     .Columbia_B
    //     .lock()
    //     .unwrap()
    //     .asset_balances
    //     .index_ref(&config::TickerSymbol::JNJ)
    //     .lock()
    //     .unwrap() = 30000;
    // *global_account_state
    //     .Columbia_B
    //     .lock()
    //     .unwrap()
    //     .net_asset_balances
    //     .index_ref(&config::TickerSymbol::JNJ)
    //     .lock()
    //     .unwrap() = 30000;

    // *global_account_state
    //     .Columbia_C
    //     .lock()
    //     .unwrap()
    //     .asset_balances
    //     .index_ref(&config::TickerSymbol::AAPL)
    //     .lock()
    //     .unwrap() = 50000;
    // *global_account_state
    //     .Columbia_C
    //     .lock()
    //     .unwrap()
    //     .net_asset_balances
    //     .index_ref(&config::TickerSymbol::AAPL)
    //     .lock()
    //     .unwrap() = 50000;
    // *global_account_state
    //     .Columbia_C
    //     .lock()
    //     .unwrap()
    //     .asset_balances
    //     .index_ref(&config::TickerSymbol::JNJ)
    //     .lock()
    //     .unwrap() = 50000;
    // *global_account_state
    //     .Columbia_C
    //     .lock()
    //     .unwrap()
    //     .net_asset_balances
    //     .index_ref(&config::TickerSymbol::JNJ)
    //     .lock()
    //     .unwrap() = 50000;
    // *global_account_state
    //     .Columbia_D
    //     .lock()
    //     .unwrap()
    //     .asset_balances
    //     .index_ref(&config::TickerSymbol::AAPL)
    //     .lock()
    //     .unwrap() = 10000;
    // *global_account_state
    //     .Columbia_D
    //     .lock()
    //     .unwrap()
    //     .net_asset_balances
    //     .index_ref(&config::TickerSymbol::AAPL)
    //     .lock()
    //     .unwrap() = 10000;
    // *global_account_state
    //     .Columbia_D
    //     .lock()
    //     .unwrap()
    //     .asset_balances
    //     .index_ref(&config::TickerSymbol::JNJ)
    //     .lock()
    //     .unwrap() = 80000;
    // *global_account_state
    //     .Columbia_D
    //     .lock()
    //     .unwrap()
    //     .net_asset_balances
    //     .index_ref(&config::TickerSymbol::JNJ)
    //     .lock()
    //     .unwrap() = 80000;

    let global_state = web::Data::new(GlobalState {
        global_orderbook_state: global_orderbook_state,
        global_account_state: global_account_state,
    });

    // logging initial global state as starting place to reconstruct state from later
    info!(
        r#"STATE DUMP: {}"#,
        serde_json::to_string(&global_state).unwrap()
    );
    
    // todo: handle flag here to allow for cli recovery

    // let gs = load_state(
    //     fs::File::open(std::path::Path::new("./test.log")).unwrap(),
    //     &web::Data::new(order_count.clone()),
    //     &web::Data::new(relay_server.clone()),
    // )
    // .unwrap();

    // info!("loaded_state: {:?}", gs);

    // global_state = web::Data::new(gs);

    // handlers discriminate based on type, so can safely pass both pieces of state here
    
    
    HttpServer::new(move || {      
        App::new().wrap(
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .supports_credentials()
            )
            .service(
            web::scope("/orders")
                .app_data(global_state.clone())
                // .app_data(web::Data::new(global_orderbook_state.clone()) // <- register the created data
                // .app_data(global_account_state.clone()) // <- register the created data
                .app_data(web::Data::new(relay_server.clone()))
                .app_data(start_time.clone())
                .app_data(web::Data::new(order_count.clone()))
                .route("/ws", web::get().to(websockets::websocket)), // .route("/grafana", web::get().to(websockets::websocket))
                                                                     // .route("/viz", web::get().to(websockets::websocket))
                                                                     // .route("/addOrder", web::post().to(add_order))
                                                                     // .route("/cancelOrder", web::post().to(cancel_order)),
        )
    })
    // todo: add multiple workers here
    // .workers(2)
    .bind(("0.0.0.0", 4000))?
    .run()
    .await
}
