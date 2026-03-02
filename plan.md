# Plan: Runtime Config, Simpler Setup, Better Hosting

## Problem

Three complaints about the current system:

1. **Config changes require recompile** — `build.rs` generates Rust code from `config.json` at compile time. Adding a trader or asset means `cargo build` + container redeploy.
2. **Complex setup** — Must manually run matching engine, two npm dev servers, and Python bots separately.
3. **Poor hosting** — Cloud Run is stateless with 1-hour connection limits.

## Solution Overview

### Runtime Config (the big change)

Replace the compile-time `build.rs` codegen with runtime config loading. The key insight: `TickerSymbol` and `TraderId` become **u16 index newtypes** backed by a global registry, instead of generated enums.

```
Before: build.rs reads config.json → generates config.rs with enums → cargo build bakes it in
After:  main.rs reads config.json at startup → populates a global OnceLock<ExchangeConfig>
```

This preserves `Copy` on all message types (critical for the matching engine's performance) and keeps Vec-based O(1) indexing (as fast or faster than the old match statements). Wire protocol stays 100% compatible.

### Docker Compose (simpler setup)

One `docker-compose up` command starts everything: matching engine, exchange client, timer, and price enforcer bot.

### Persistent Server

Volume-mounted config means you just edit `config.json` and restart the container — no rebuild needed. Moving off Cloud Run to any Docker host (VPS, GCE instance, etc.) solves the 1-hour connection limit.

---

## Changes by File

### Matching Engine (Rust)

| File | Change |
|------|--------|
| `src/config.rs` | **Complete rewrite.** Replace macro-generated enums/structs with: `TickerSymbol(u16)` and `TraderId(u16)` newtypes, `OnceLock<ExchangeConfig>` global registry, `load_config_from_file()`, Vec-backed `AssetBalances`/`GlobalOrderBookState`/`GlobalAccountState` with identical `index_ref()` signatures, custom Serde for wire compat |
| `src/main.rs` | Add 2 lines: read `CONFIG_PATH` env var, call `init_config()` before state construction |
| `src/accounts.rs` | Remove `strum` import, move `Password` type to config.rs, add re-export |
| `src/orderbook.rs` | Replace `!= TraderId::Price_Enforcer` → `.is_price_enforcer()` (2 sites) |
| `src/websockets.rs` | Replace `!= TraderId::Price_Enforcer` → `.is_price_enforcer()` (6 sites) |
| `src/controls.rs` | Remove `strum` import, replace Price_Enforcer comparison (1 site) |
| `Cargo.toml` | Remove `build = "build.rs"`, `[build-dependencies]`, `strum`, `strum_macros`, `paste` |
| `build.rs` | **Delete** |
| `Dockerfile` | Remove build.rs from COPY, add `ENV CONFIG_PATH=/app/config.json` |

### New Files

| File | Purpose |
|------|---------|
| `docker-compose.yml` | Orchestrates all 4 services |
| `exchange-client/Dockerfile` | node:20-slim, npm ci, vite dev server |
| `timer/Dockerfile` | node:20-slim, npm ci, vite dev server |
| `python_bots/Dockerfile` | python:3.11-slim, pip install deps, run bot |

### Timer

- Make server URL configurable via env var instead of hardcoded Cloud Run URL

---

## How Config Loading Works (New Architecture)

```
config.json (unchanged format)
    ↓
load_config_from_file() at startup
    ↓
OnceLock<ExchangeConfig> (global, immutable after init)
  ├── ticker_names: ["AD", "TS", "TT"]
  ├── trader_names: ["Price_Enforcer", "zev", "trader1", ...]
  ├── trader_passwords: [['p','e','n','f'], ['0','0','0','0'], ...]
  ├── start_cents_balance: 10000
  ├── start_asset_balance: 100
  ├── max_price_cents: 101
  └── price_enforcer_id: TraderId(0)

TickerSymbol(0) = "AD",  TickerSymbol(1) = "TS",  TickerSymbol(2) = "TT"
TraderId(0) = "Price_Enforcer",  TraderId(1) = "zev",  ...
```

- `TickerSymbol::from_str("AD")` → scans vec → `TickerSymbol(0)`
- `TickerSymbol(0).name()` → `"AD"`
- `global_orderbook_state.index_ref(&TickerSymbol(0))` → `&books[0]` (Vec index)
- Serialize `TickerSymbol(0)` → `"AD"` (custom Serde)

---

## Docker Compose Architecture

```
docker-compose up
    ├── matching-engine (port 8080)
    │   └── volumes: ./matching-engine/config.json:/app/config.json
    ├── exchange-client (port 5173)
    ├── timer (port 3000)
    └── price-enforcer
        └── connects to ws://matching-engine:8080/orders/ws
```

To change config: edit `config.json`, then `docker-compose restart matching-engine`.

---

## Verification Plan

1. `cargo build --release` — compiles without build.rs
2. `cargo run` — loads config.json at runtime, starts server
3. Edit config.json (add a trader), restart — works without recompile
4. `docker-compose up --build` — all services start
5. Exchange client connects and trades normally (wire protocol unchanged)
6. Python price enforcer connects and trades
