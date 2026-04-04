use std::sync::OnceLock;
use std::sync::Mutex;
use std::str::FromStr;
use std::fmt;
use std::fs::File;
use std::io::BufReader;

use serde::{Deserialize, Serialize, Deserializer, Serializer};

use crate::accounts::TraderAccount;
use crate::orderbook::OrderBook;

pub type TraderIp = std::net::Ipv4Addr;

// ---------------------------------------------------------------------------
// Global config registry
// ---------------------------------------------------------------------------

static CONFIG: OnceLock<ExchangeConfig> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct RawConfig {
    start_asset_balance: i64,
    start_cents_balance: usize,
    assets: Vec<RawAsset>,
    accounts: Vec<RawAccount>,
}

#[derive(Debug, Deserialize)]
struct RawAsset {
    symbol: String,
}

#[derive(Debug, Deserialize)]
struct RawAccount {
    trader_id: String,
    password: String,
}

#[derive(Debug)]
pub struct ExchangeConfig {
    pub ticker_names: Vec<String>,
    pub trader_names: Vec<String>,
    pub trader_passwords: Vec<[char; 4]>,
    pub start_cents_balance: usize,
    pub start_asset_balance: i64,
    pub price_enforcer_id: TraderId,
}

pub fn init_config(path: &str) {
    let file = File::open(path).expect("Failed to open config file");
    let reader = BufReader::new(file);
    let raw: RawConfig = serde_json::from_reader(reader).expect("Failed to parse config.json");

    let ticker_names: Vec<String> = raw.assets.iter().map(|a| a.symbol.clone()).collect();
    let trader_names: Vec<String> = raw.accounts.iter().map(|a| a.trader_id.clone()).collect();
    let trader_passwords: Vec<[char; 4]> = raw
        .accounts
        .iter()
        .map(|a| {
            let chars: Vec<char> = a.password.chars().collect();
            chars
                .try_into()
                .expect("Password must be exactly 4 characters")
        })
        .collect();

    let price_enforcer_idx = trader_names
        .iter()
        .position(|n| n == "Price_Enforcer")
        .expect("Config must contain a Price_Enforcer account");

    let cfg = ExchangeConfig {
        ticker_names,
        trader_names,
        trader_passwords,
        start_cents_balance: raw.start_cents_balance,
        start_asset_balance: raw.start_asset_balance,
        price_enforcer_id: TraderId(price_enforcer_idx as u16),
    };

    CONFIG.set(cfg).expect("Config already initialised");
}

pub fn config() -> &'static ExchangeConfig {
    CONFIG.get().expect("Config not initialised — call init_config() first")
}

// ---------------------------------------------------------------------------
// TickerSymbol  –  u16 newtype
// ---------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickerSymbol(pub u16);

impl TickerSymbol {
    pub fn name(&self) -> &'static str {
        &config().ticker_names[self.0 as usize]
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.name().as_bytes()
    }

    pub fn count() -> usize {
        config().ticker_names.len()
    }

    pub fn all() -> Vec<TickerSymbol> {
        (0..Self::count()).map(|i| TickerSymbol(i as u16)).collect()
    }
}

impl FromStr for TickerSymbol {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        config()
            .ticker_names
            .iter()
            .position(|n| n == s)
            .map(|i| TickerSymbol(i as u16))
            .ok_or("Invalid ticker symbol")
    }
}

impl fmt::Display for TickerSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Serialize for TickerSymbol {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for TickerSymbol {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        TickerSymbol::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// TraderId  –  u16 newtype
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraderId(pub u16);

impl TraderId {
    pub fn name(&self) -> &'static str {
        &config().trader_names[self.0 as usize]
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.name().as_bytes()
    }

    pub fn is_price_enforcer(&self) -> bool {
        *self == config().price_enforcer_id
    }

    pub fn count() -> usize {
        config().trader_names.len()
    }

    pub fn all() -> Vec<TraderId> {
        (0..Self::count()).map(|i| TraderId(i as u16)).collect()
    }
}

impl FromStr for TraderId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        config()
            .trader_names
            .iter()
            .position(|n| n == s)
            .map(|i| TraderId(i as u16))
            .ok_or("Invalid trader ID")
    }
}

impl fmt::Debug for TraderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl fmt::Display for TraderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Serialize for TraderId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for TraderId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        TraderId::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// AssetBalances  –  Vec<Mutex<i64>>
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct AssetBalances {
    balances: Vec<Mutex<i64>>,
}

impl AssetBalances {
    pub fn new() -> Self {
        let n = TickerSymbol::count();
        Self {
            balances: (0..n).map(|_| Mutex::new(0)).collect(),
        }
    }

    pub fn index_ref(&self, symbol: &TickerSymbol) -> &Mutex<i64> {
        &self.balances[symbol.0 as usize]
    }
}

impl Serialize for AssetBalances {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.balances.len()))?;
        for (i, m) in self.balances.iter().enumerate() {
            let sym = TickerSymbol(i as u16);
            let val = *m.lock().unwrap();
            map.serialize_entry(sym.name(), &val)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for AssetBalances {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: std::collections::HashMap<String, i64> =
            std::collections::HashMap::deserialize(deserializer)?;
        let n = TickerSymbol::count();
        let balances: Vec<Mutex<i64>> = (0..n).map(|_| Mutex::new(0)).collect();
        for (name, val) in map {
            if let Ok(sym) = TickerSymbol::from_str(&name) {
                *balances[sym.0 as usize].lock().unwrap() = val;
            }
        }
        Ok(AssetBalances { balances })
    }
}

// ---------------------------------------------------------------------------
// GlobalOrderBookState  –  Vec<Mutex<OrderBook>>
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct GlobalOrderBookState {
    books: Vec<Mutex<OrderBook>>,
}

impl GlobalOrderBookState {
    pub fn new() -> Self {
        let cfg = config();
        let books = TickerSymbol::all()
            .into_iter()
            .map(|sym| Mutex::new(crate::orderbook::quickstart_order_book(sym)))
            .collect();
        Self { books }
    }

    pub fn index_ref(&self, symbol: &TickerSymbol) -> &Mutex<OrderBook> {
        &self.books[symbol.0 as usize]
    }
}

impl Serialize for GlobalOrderBookState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.books.len()))?;
        for (i, m) in self.books.iter().enumerate() {
            let sym = TickerSymbol(i as u16);
            let book = m.lock().unwrap();
            map.serialize_entry(sym.name(), &*book)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for GlobalOrderBookState {
    fn deserialize<D: Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new())
    }
}

// ---------------------------------------------------------------------------
// GlobalAccountState  –  Vec<Mutex<TraderAccount>>
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct GlobalAccountState {
    accounts: Vec<Mutex<TraderAccount>>,
}

impl GlobalAccountState {
    pub fn new() -> Self {
        let cfg = config();
        let accounts = TraderId::all()
            .into_iter()
            .map(|id| {
                let pw = cfg.trader_passwords[id.0 as usize];
                Mutex::new(crate::accounts::quickstart_trader_account(
                    id,
                    cfg.start_cents_balance,
                    cfg.start_asset_balance,
                    pw,
                ))
            })
            .collect();
        Self { accounts }
    }

    pub fn index_ref(&self, account_id: TraderId) -> &Mutex<TraderAccount> {
        &self.accounts[account_id.0 as usize]
    }
}

impl Serialize for GlobalAccountState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.accounts.len()))?;
        for (i, m) in self.accounts.iter().enumerate() {
            let id = TraderId(i as u16);
            let acct = m.lock().unwrap();
            map.serialize_entry(id.name(), &*acct)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for GlobalAccountState {
    fn deserialize<D: Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new())
    }
}
