use std::collections::HashMap;

use actix::Handler;
use actix_broker::{Broker, SystemBroker};
use actix_web::{web, HttpResponse};
use strum::IntoEnumIterator;
use actix_web::Error;
use crate::{config::{self, TraderId}, message_types::{GameEndMessage, GameStartedMessage}, websockets::MyWebSocketActor, GlobalState};

impl Handler<GameStartedMessage> for MyWebSocketActor {
    type Result = ();

    fn handle(&mut self, msg: GameStartedMessage, ctx: &mut Self::Context) {
        if msg.0 == "GameStarted" {
            ctx.text(format!("{{\"GameStartedMessage\" : \"GameStarted\"}}"));
        }
    }
}


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

    let orderbooks = &global_state.global_orderbook_state;
    let mut final_prices = Vec::new();
    
    for symbol in config::TickerSymbol::iter() {
        let orderbook = orderbooks.index_ref(&symbol).lock().unwrap();
        if let Some(last_trade) = orderbook.price_history.last() {
            final_prices.push((symbol, last_trade.1));
        }
    }

    let accounts = &global_state.global_account_state;
    let mut final_standings = Vec::new();

    for trader_id in config::TraderId::iter() {
        if trader_id != TraderId::Price_Enforcer {
            let account = accounts.index_ref(trader_id).lock().unwrap();
            let mut total_value = account.cents_balance;

            for (symbol, price) in &final_prices {
                let asset_balance = *account.asset_balances.index_ref(symbol).lock().unwrap();
                let asset_value =  (*price as usize) * (100 * (asset_balance as usize)) / (100 + (asset_balance as usize));
                total_value += asset_value;
            }

            final_standings.push((trader_id, total_value));
        }
    }

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