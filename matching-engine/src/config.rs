use std::collections::HashMap;
use std::net::Ipv4Addr;
pub type TraderIp = std::net::Ipv4Addr;
use std::io;
use actix::Addr;
use crate::websockets::MyWebSocketActor;
pub use crate::accounts::quickstart_trader_account;
pub use crate::orderbook::quickstart_order_book;

use strum_macros::EnumIter;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;
use crate::accounts::TraderAccount;
use crate::orderbook::OrderBook;
use std::str::FromStr;

macro_rules! generate_ticker_enum {
    ([$($name:ident),*]) => {
        #[derive(Debug, Copy, Clone, Deserialize, Serialize)]
        pub enum TickerSymbol {
            $($name, )*
        }
        impl TryFrom<&'static str> for TickerSymbol {
            type Error = &'static str;

            fn try_from(s: &'static str) -> Result<TickerSymbol, &'static str> {
                match s {
                    $(stringify!($name) => Ok(TickerSymbol::$name),)+
                    _ => Err("Invalid String")
                }
            }
        }

        impl FromStr for TickerSymbol {
            type Err = &'static str;
        
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(stringify!($name) => Ok(TickerSymbol::$name),)+
                    _ => Err("Invalid String")
                }
            }
        }

        impl TickerSymbol {
            //type Err = &'static str;

            pub fn as_bytes(&self) -> &[u8] {
                match &self {
                    $(TickerSymbol::$name => stringify!($name).as_bytes(),)+
                    //_ => Err("Invalid String")
                }
            }
        }
        
    };
}

macro_rules! generate_accounts_enum {
    ([$($name:ident),*]) => {
        #[derive(Debug, Copy, Clone, Deserialize, Serialize, EnumIter, PartialEq)]
        pub enum TraderId {
            $($name, )*
        }
        impl TryFrom<&'static str> for TraderId {
            type Error = &'static str;

            fn try_from(s: &'static str) -> Result<TraderId, &'static str> {
                match s {
                    $(stringify!($name) => Ok(TraderId::$name),)+
                    _ => Err("Invalid String")
                }
            }            
        }    

        impl FromStr for TraderId {
            type Err = &'static str;
        
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(stringify!($name) => Ok(TraderId::$name),)+
                    _ => Err("Invalid String")
                }
            }
        }

        impl TraderId {
            //type Err = &'static str;

            pub fn as_bytes(&self) -> &[u8] {
                match &self {
                    $(TraderId::$name => stringify!($name).as_bytes(),)+
                    //_ => Err("Invalid String")
                }
            }
        }   
    };
}

macro_rules! generate_account_balances_struct {
    ([$($name:ident),*]) => {
        #[derive(Debug, Serialize, Deserialize)]
        pub struct AssetBalances {
            $($name: Mutex<i64>, )*
        }    

        impl AssetBalances {
            pub fn index_ref (&self, symbol:&TickerSymbol) -> &Mutex<i64>{
                match symbol {
                    $(TickerSymbol::$name => {&self.$name}, )*
                }
            }     
            
            pub fn new() -> Self {
                Self { 
                    $($name: Mutex::new(0), )*
                 }
            }
               
        }
    };
}

macro_rules! generate_global_state {
    ([$($name:ident),*], [$($account_id:ident),*]) => {
        #[derive(Debug, Serialize, Deserialize)]
        pub struct GlobalOrderBookState {
            $(pub $name: Mutex<crate::orderbook::OrderBook>, )*
        }
        
        impl GlobalOrderBookState {
            pub fn index_ref (&self, symbol:&TickerSymbol) -> &Mutex<crate::orderbook::OrderBook>{
                match symbol {
                    $(TickerSymbol::$name => {&self.$name}, )*
                }
            }

        }
        
        #[derive(Debug, Serialize, Deserialize)]
        pub struct GlobalAccountState {
            $(pub $account_id: Mutex<crate::accounts::TraderAccount>, )*
        }

        impl GlobalAccountState {
            pub fn index_ref (&self, account_id:crate::config::TraderId,) -> &Mutex<crate::accounts::TraderAccount>{
                match account_id {
                    $(TraderId::$account_id => {&self.$account_id}, )*
                }
            }       
                    
        }

    };

}

macro_rules! init_orderbook {
([$($value:ident),+]) => {
    GlobalOrderBookState {
        $($value: Mutex::new(quickstart_order_book(TickerSymbol::$value,0,100,10000)), )*
    }
    };
}

macro_rules! init_accounts {
([$(($username:ident, $password:expr)),*]) => {
    GlobalAccountState {
        $($username: Mutex::new(quickstart_trader_account(
            TraderId::$username,
            10000,
            $password.chars().collect::<Vec<_>>().try_into().unwrap(),
        )), )*
    }
    };
}


generate_ticker_enum!([AD,TS,TT]);
generate_account_balances_struct!([AD,TS,TT]);
generate_global_state!([AD,TS,TT], [Price_Enforcer,zev,TEST7,cu_b,cu_c,cu_d,cu_e,cu_f,cu_g,cu_h,cu_i,cu_j,cu_k,cu_l,cu_m,cu_n,cu_o,cu_p,cu_q,cu_r,cu_s]);
generate_accounts_enum!([Price_Enforcer,zev,TEST7,cu_b,cu_c,cu_d,cu_e,cu_f,cu_g,cu_h,cu_i,cu_j,cu_k,cu_l,cu_m,cu_n,cu_o,cu_p,cu_q,cu_r,cu_s]);


impl GlobalOrderBookState {
        pub fn new() -> Self {
            init_orderbook!([AD,TS,TT])
        }
}

impl GlobalAccountState {
        pub fn new() -> Self {
            init_accounts!([(Price_Enforcer,"penf"),(zev,"0000"),(TEST7,"0001"),(cu_b,"0002"),(cu_c,"0003"),(cu_d,"0004"),(cu_e,"0005"),(cu_f,"0006"),(cu_g,"0007"),(cu_h,"0008"),(cu_i,"0009"),(cu_j,"0010"),(cu_k,"0011"),(cu_l,"0012"),(cu_m,"0013"),(cu_n,"0014"),(cu_o,"0015"),(cu_p,"0016"),(cu_q,"0017"),(cu_r,"0018"),(cu_s,"0019")])
        }
}