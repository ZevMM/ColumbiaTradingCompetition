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
        #[derive(Debug, Copy, Clone, Deserialize, Serialize, EnumIter)]
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
        #[derive(Debug, Copy, Clone, Deserialize, Serialize, EnumIter, PartialEq, Eq, PartialOrd, Ord)]
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
            100000,
            100000,
            $password.chars().collect::<Vec<_>>().try_into().unwrap()
        )), )*
    }
    };
}


generate_ticker_enum!([AD,TS,TT]);
generate_account_balances_struct!([AD,TS,TT]);
generate_global_state!([AD,TS,TT], [Price_Enforcer,zev,trader1,trader2,trader3,trader4,trader5,trader6,trader7,trader8,trader9,trader10,trader11,trader12,trader13,trader14,trader15,trader16,trader17,trader18,trader19,trader20,trader21,trader22,trader23,trader24,trader25,trader26,trader27,trader28,trader29,trader30,trader31,trader32,trader33,trader34,trader35,trader36,trader37,trader38,trader39,trader40,trader41,trader42,trader43,trader44,trader45,trader46,trader47,trader48,trader49,trader50,trader51,trader52,trader53,trader54,trader55,trader56,trader57,trader58,trader59,trader60,trader61,trader62,trader63,trader64,trader65,trader66,trader67,trader68,trader69,trader70,trader71,trader72,trader73,trader74,trader75,trader76,trader77,trader78,trader79,trader80,trader81,trader82,trader83,trader84,trader85,trader86,trader87,trader88,trader89,trader90,trader91,trader92,trader93,trader94,trader95,trader96,trader97,trader98,trader99,trader100]);
generate_accounts_enum!([Price_Enforcer,zev,trader1,trader2,trader3,trader4,trader5,trader6,trader7,trader8,trader9,trader10,trader11,trader12,trader13,trader14,trader15,trader16,trader17,trader18,trader19,trader20,trader21,trader22,trader23,trader24,trader25,trader26,trader27,trader28,trader29,trader30,trader31,trader32,trader33,trader34,trader35,trader36,trader37,trader38,trader39,trader40,trader41,trader42,trader43,trader44,trader45,trader46,trader47,trader48,trader49,trader50,trader51,trader52,trader53,trader54,trader55,trader56,trader57,trader58,trader59,trader60,trader61,trader62,trader63,trader64,trader65,trader66,trader67,trader68,trader69,trader70,trader71,trader72,trader73,trader74,trader75,trader76,trader77,trader78,trader79,trader80,trader81,trader82,trader83,trader84,trader85,trader86,trader87,trader88,trader89,trader90,trader91,trader92,trader93,trader94,trader95,trader96,trader97,trader98,trader99,trader100]);


impl GlobalOrderBookState {
        pub fn new() -> Self {
            init_orderbook!([AD,TS,TT])
        }
}

impl GlobalAccountState {
        pub fn new() -> Self {
            init_accounts!([(Price_Enforcer,"penf"),(zev,"0000"),(trader1,"0001"),(trader2,"0002"),(trader3,"0003"),(trader4,"0004"),(trader5,"0005"),(trader6,"0006"),(trader7,"0007"),(trader8,"0008"),(trader9,"0009"),(trader10,"0010"),(trader11,"0011"),(trader12,"0012"),(trader13,"0013"),(trader14,"0014"),(trader15,"0015"),(trader16,"0016"),(trader17,"0017"),(trader18,"0018"),(trader19,"0019"),(trader20,"0020"),(trader21,"0021"),(trader22,"0022"),(trader23,"0023"),(trader24,"0024"),(trader25,"0025"),(trader26,"0026"),(trader27,"0027"),(trader28,"0028"),(trader29,"0029"),(trader30,"0030"),(trader31,"0031"),(trader32,"0032"),(trader33,"0033"),(trader34,"0034"),(trader35,"0035"),(trader36,"0036"),(trader37,"0037"),(trader38,"0038"),(trader39,"0039"),(trader40,"0040"),(trader41,"0041"),(trader42,"0042"),(trader43,"0043"),(trader44,"0044"),(trader45,"0045"),(trader46,"0046"),(trader47,"0047"),(trader48,"0048"),(trader49,"0049"),(trader50,"0050"),(trader51,"0051"),(trader52,"0052"),(trader53,"0053"),(trader54,"0054"),(trader55,"0055"),(trader56,"0056"),(trader57,"0057"),(trader58,"0058"),(trader59,"0059"),(trader60,"0060"),(trader61,"0061"),(trader62,"0062"),(trader63,"0063"),(trader64,"0064"),(trader65,"0065"),(trader66,"0066"),(trader67,"0067"),(trader68,"0068"),(trader69,"0069"),(trader70,"0070"),(trader71,"0071"),(trader72,"0072"),(trader73,"0073"),(trader74,"0074"),(trader75,"0075"),(trader76,"0076"),(trader77,"0077"),(trader78,"0078"),(trader79,"0079"),(trader80,"0080"),(trader81,"0081"),(trader82,"0082"),(trader83,"0083"),(trader84,"0084"),(trader85,"0085"),(trader86,"0086"),(trader87,"0087"),(trader88,"0088"),(trader89,"0089"),(trader90,"0090"),(trader91,"0091"),(trader92,"0092"),(trader93,"0093"),(trader94,"0094"),(trader95,"0095"),(trader96,"0096"),(trader97,"0097"),(trader98,"0098"),(trader99,"0099"),(trader100,"0100")])
        }
}