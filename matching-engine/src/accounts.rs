use crate::api_messages::JsonPayload;
use crate::config;
use crate::orderbook::Order;
use crate::orderbook::OrderID;
use crate::websockets;
use actix::Addr;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;


pub type Password = [char; 4];
#[derive(Debug, Serialize, Deserialize)]
pub struct TraderAccount {
    pub trader_id: config::TraderId,
    pub cents_balance: usize,
    #[serde(skip, default = "ret_none")]
    pub current_actor: Option<Addr<websockets::MyWebSocketActor>>,
    pub password: Password,

    // Active orders keyed by order ID for O(1) lookup/update on fills and cancels.
    pub active_orders: HashMap<OrderID, Order>,

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
    /// Push a pre-serialized JSON fill notification to the trader's WebSocket actor.
    pub fn push_fill(&mut self, fill_event: JsonPayload) {
        if let Some(addr) = &self.current_actor {
            let _ = addr.try_send(fill_event);
        }
    }
}

pub fn quickstart_trader_account(
    trader_id: config::TraderId,
    cents_balance: usize,
    start_asset_balance: i64,
    password: Password,
) -> TraderAccount {
    let mut asset_balances = config::AssetBalances::new();
    let mut net_asset_balances = config::AssetBalances::new();

    // making it just give the same number of shares for each asset cus I feel lazy
    for symbol in config::TickerSymbol::all() {
        *asset_balances.index_ref_mut(&symbol) = start_asset_balance;
        *net_asset_balances.index_ref_mut(&symbol) = start_asset_balance;
    }

    TraderAccount {
        trader_id,
        cents_balance,
        net_cents_balance: cents_balance,
        asset_balances,
        net_asset_balances,
        current_actor: None,
        password,
        active_orders: HashMap::with_capacity(256),
    }
}