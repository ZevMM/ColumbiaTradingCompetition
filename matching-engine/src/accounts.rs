use crate::config;
use crate::orderbook::Order;
use crate::websockets;
use actix::Addr;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use strum::IntoEnumIterator;


pub type Password = [char; 4];
#[derive(Debug, Serialize, Deserialize)]
pub struct TraderAccount {
    pub trader_id: config::TraderId,
    pub cents_balance: usize,
    #[serde(skip, default = "ret_none")]
    pub current_actor: Option<Addr<websockets::MyWebSocketActor>>,
    pub password: Password,

    // all active orders for syncing purposes. Maybe should be organized by symbol/price level to allow for quicker removal/access?
    pub active_orders: Vec<Order>,

    pub net_cents_balance: usize,
    // asset_balances, net_asset_balances updated on fill event, and so should be current
    // in asset lots
    pub asset_balances: config::AssetBalances,
    // in shares, equal to the total of owned shares minus the total of outstanding sell orders' shares (i.e. should be \geq 0)
    pub net_asset_balances: config::AssetBalances,
}

fn ret_none() -> Option<Addr<websockets::MyWebSocketActor>> {
    None
}

impl TraderAccount {
    pub fn push_fill(&mut self, fill_event: Arc<crate::api_messages::OrderFillMessage>) {
        if let Some(addr) = &self.current_actor {
            match addr.try_send(fill_event) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }
}

pub fn quickstart_trader_account(
    trader_id: config::TraderId,
    cents_balance: usize,
    start_asset_balance: i64,
    password: Password,
) -> TraderAccount {
    let asset_balances = config::AssetBalances::new();
    let net_asset_balances = config::AssetBalances::new();

    // making it just give the same number of shares for each asset cus I feel lazy
    for symbol in config::TickerSymbol::iter() {
        *asset_balances.index_ref(&symbol).lock().unwrap() = start_asset_balance;
        *net_asset_balances.index_ref(&symbol).lock().unwrap() = start_asset_balance;
    }

    TraderAccount {
        trader_id,
        cents_balance,
        net_cents_balance: cents_balance,
        asset_balances: config::AssetBalances::new(),
        net_asset_balances: config::AssetBalances::new(),
        current_actor: None,
        password,
        active_orders: Vec::with_capacity(10000),
    }
}
