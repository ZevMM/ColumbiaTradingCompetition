use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Mutex;
extern crate env_logger;
use actix::Actor;

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
mod controls;
mod datastream;

use std::sync::atomic::AtomicUsize;

use std::time::SystemTime;

use std::sync::Arc;

pub use crate::accounts::TraderAccount;
pub use crate::accounts::quickstart_trader_account;
pub use crate::orderbook::quickstart_order_book;
pub use crate::orderbook::OrderBook;
pub use crate::orderbook::OrderType;

use config::GlobalAccountState;
use config::GlobalOrderBookState;


#[derive(Debug, Serialize, Deserialize)]
struct GlobalState {
    global_orderbook_state: GlobalOrderBookState,
    global_account_state: GlobalAccountState,
    #[serde(skip)]
    game_started: Arc<Mutex<bool>>, // Shared flag to track game state
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init_timed();

    let start_time = web::Data::new(SystemTime::now());
    let order_counter = web::Data::new(Arc::new(AtomicUsize::new(0)));
    let relay_server = web::Data::new(connection_server::Server::new().start());
    let global_state = web::Data::new(GlobalState {
        global_orderbook_state: config::GlobalOrderBookState::new(),
        global_account_state: config::GlobalAccountState::new(),
        game_started: Arc::new(Mutex::new(false)),
    });

    println!("Starting matching engine server...");

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::default().allow_any_origin().allow_any_method().allow_any_header())
            .app_data(global_state.clone())
            .app_data(start_time.clone())
            .app_data(relay_server.clone())
            .app_data(order_counter.clone())
            .route("/start_game", web::get().to(controls::start_game))
            .route("/end_game", web::get().to(controls::end_game))
            .route("/tally_score", web::get().to(controls::tally_score))
            .route("/orders/ws", web::get().to(websockets::websocket))
            .route("/market_data/ws", web::get().to(datastream::market_data_websocket))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
