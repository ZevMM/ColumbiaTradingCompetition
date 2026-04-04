# Trading Competition — WebSocket API Reference

This document covers everything you need to connect an algorithmic trading bot to the exchange.

---

## Endpoints

| Endpoint | Auth required | Use |
|---|---|---|
| `ws://<host>:8080/orders/ws` | Yes | Place/cancel orders + receive all market data |
| `ws://<host>:8080/market_data/ws` | No | Read-only market data feed |

---

## Read-Only Market Data Feed

Connect to `ws://<host>:8080/market_data/ws` with no credentials:

```python
async with websockets.connect("ws://localhost:8080/market_data/ws") as ws:
    async for raw in ws:
        print(json.loads(raw))
```

On connect you receive the full `GameState` snapshot. After that you receive `TradeOccurredMessage`, `NewRestingOrderMessage`, and `CancelOccurredMessage` in real time. Personal messages (fills, order confirms) are not sent. Sending any message returns an error.

Use this if you only want to observe the market without trading.

---

## Trading Connection

Connect via WebSocket to:

```
ws://<host>:8080/orders/ws
```

**Authentication** is done at connection time using the `Sec-WebSocket-Protocol` header:

```
Sec-WebSocket-Protocol: <trader_id>|<password>
```

Example: if your trader ID is `trader42` and your password is `0042`, the header value is `trader42|0042`.

In Python with the `websockets` library:

```python
import asyncio
import websockets
import json

async def main():
    uri = "ws://localhost:8080/orders/ws"
    async with websockets.connect(uri, subprotocols=["trader42|0042"]) as ws:
        async for message in ws:
            data = json.loads(message)
            print(data)

asyncio.run(main())
```

**On successful connection** the server immediately sends two messages:
1. Your current account info (`AccountInfo`)
2. The full current game state (`GameState`)

---

## Rate Limit

Non-bot connections are limited to **10 order/cancel requests per second** per connection (sliding 1-second window). Requests that exceed this limit receive a `OrderPlaceErrorMessage` or `CancelErrorMessage` with `"Rate limit exceeded"` and are dropped — your connection stays open.

---

## Message Format

All messages are JSON. Every message sent **to** the server must include a `"MessageType"` field that identifies the message type.

---

## Client → Server Messages

### Place an Order — `OrderRequest`

```json
{
    "MessageType": "OrderRequest",
    "OrderType": "Buy",
    "Amount": 10,
    "Price": 45,
    "Symbol": "TT",
    "TraderId": "trader42",
    "Password": ["0", "0", "4", "2"]
}
```

| Field | Type | Description |
|---|---|---|
| `MessageType` | string | Must be `"OrderRequest"` |
| `OrderType` | string | `"Buy"` or `"Sell"` |
| `Amount` | integer | Number of shares (1 – 10,000) |
| `Price` | integer | Limit price in cents (≥ 1) |
| `Symbol` | string | Ticker symbol, e.g. `"TT"`, `"TS"`, `"AD"` |
| `TraderId` | string | Your trader ID |
| `Password` | char array | Your 4-character password as an array of single-character strings |

**Constraints:**
- `Amount` must be between 1 and 10,000 inclusive
- `Price` must be between 1 and 1,000,000 inclusive
- `TraderId` must match your authenticated connection
- For **Buy** orders: you must have sufficient cash (`net_cents_balance ≥ Amount × Price`)
- For **Sell** orders: you must have sufficient shares (`net_asset_balances[Symbol] ≥ Amount`)

**Note on Password format:** the password is sent as a JSON array of individual characters, not a plain string. For password `"0042"` send `["0", "0", "4", "2"]`.

---

### Cancel an Order — `CancelRequest`

```json
{
    "MessageType": "CancelRequest",
    "OrderId": 1234,
    "TraderId": "trader42",
    "Price": 45,
    "Symbol": "TT",
    "Side": "Buy",
    "Password": ["0", "0", "4", "2"]
}
```

| Field | Type | Description |
|---|---|---|
| `MessageType` | string | Must be `"CancelRequest"` |
| `OrderId` | integer | The order ID returned in the original `OrderConfirmMessage` |
| `TraderId` | string | Your trader ID |
| `Price` | integer | Price of the order to cancel |
| `Symbol` | string | Symbol of the order to cancel |
| `Side` | string | `"Buy"` or `"Sell"` |
| `Password` | char array | Your 4-character password |

---

### Request Account Info — `AccountInfoRequest`

Requests a snapshot of your current account state (balances, active orders, etc.).

```json
{
    "MessageType": "AccountInfoRequest",
    "TraderId": "trader42",
    "Password": ["0", "0", "4", "2"]
}
```

---

### Request Game State — `GameStateRequest`

Requests the full current order book state for all symbols.

```json
{
    "MessageType": "GameStateRequest"
}
```

---

## Server → Client Messages

All server messages are JSON objects. Use the top-level key to identify the message type.

---

### `AccountInfo`

Sent automatically on connection, and in response to `AccountInfoRequest`.

```json
{
    "AccountInfo": {
        "trader_id": "trader42",
        "cents_balance": 10000,
        "net_cents_balance": 8750,
        "net_asset_balances": {
            "TT": 3,
            "TS": 0,
            "AD": 5
        },
        "active_orders": [...]
    }
}
```

- `cents_balance` — total cash deposited (does not decrease when placing orders)
- `net_cents_balance` — cash available for new buy orders (decreases when an order is placed, increases on cancel or fill)
- `net_asset_balances` — shares available to sell per symbol
- `active_orders` — list of your open orders currently resting on the book

---

### `GameState`

Sent automatically on connection, and in response to `GameStateRequest`. Contains the full order book for every symbol.

```json
{
    "GameState": {
        "TT": {
            "symbol": "TT",
            "buy_side": { "44": 50, "43": 120 },
            "sell_side": { "46": 30, "47": 80 },
            "price_history": [[1720000000, 45, 10], ...]
        },
        "TS": { ... },
        "AD": { ... }
    }
}
```

- `buy_side` / `sell_side` — objects mapping price (as string) → total volume at that level
- `price_history` — array of `[unix_timestamp, price, volume]` tuples for every trade that occurred

---

### `OrderConfirmMessage`

Sent to you when your order is accepted and resting on the book.

```json
{
    "OrderConfirmMessage": {
        "order_info": {
            "order_id": 1234,
            "trader_id": "trader42",
            "symbol": "TT",
            "amount": 10,
            "price": 45,
            "order_type": "Buy"
        }
    }
}
```

Save `order_id` — you need it to cancel the order later.

---

### `OrderPlaceErrorMessage`

Sent to you when your order was rejected.

```json
{
    "OrderPlaceErrorMessage": {
        "side": "Buy",
        "price": 45,
        "symbol": "TT",
        "error_details": "Error Placing Order: The total value of order is greater than current account balance"
    }
}
```

Common `error_details` values:
- `"Amount must be greater than zero"`
- `"Volume exceeds maximum allowed single-order volume"` (> 10,000 shares)
- `"Price must be greater than zero"`
- `"Price exceeds maximum allowed value"` (> 1,000,000)
- `"Price level is at capacity"`
- `"Trader has reached maximum number of active orders"`
- `"Error Placing Order: The total value of order is greater than current account balance"`
- `"Error Placing Order: The total amount of this trade would take your account short"`
- `"Rate limit exceeded"`

---

### `CancelConfirmMessage`

Sent to you when your cancel is successful.

```json
{
    "CancelConfirmMessage": {
        "order_info": {
            "order_id": 1234,
            "trader_id": "trader42",
            "symbol": "TT",
            "amount": 10,
            "price": 45,
            "order_type": "Buy"
        }
    }
}
```

---

### `CancelErrorMessage`

Sent to you when a cancel request failed.

```json
{
    "CancelErrorMessage": {
        "order_id": 1234,
        "side": "Buy",
        "price": 45,
        "symbol": "TT",
        "error_details": "order_id not found at specified price/side"
    }
}
```

---

### `OrderFillMessage`

Sent to you when one of your resting orders is (partially) filled.

```json
{
    "OrderFillMessage": {
        "order_id": 1234,
        "amount_filled": 5,
        "price": 45
    }
}
```

If `amount_filled` is less than the original order amount, the remainder stays on the book.

---

### `TradeOccurredMessage`

Broadcast to **all** connected clients whenever a trade occurs.

```json
{
    "TradeOccurredMessage": {
        "amount": 5,
        "symbol": "TT",
        "resting_side": "Buy",
        "price": 45,
        "time": 1720000123
    }
}
```

- `resting_side` — side of the order that was already on the book (the passive side)
- `time` — Unix timestamp in seconds

---

### `NewRestingOrderMessage`

Broadcast to **all** connected clients when an order is placed and rests on the book (no immediate fill).

```json
{
    "NewRestingOrderMessage": {
        "side": "Buy",
        "amount": 10,
        "symbol": "TT",
        "price": 45
    }
}
```

---

### `CancelOccurredMessage`

Broadcast to **all** connected clients when an order is cancelled.

```json
{
    "CancelOccurredMessage": {
        "side": "Buy",
        "amount": 10,
        "symbol": "TT",
        "price": 45
    }
}
```

---

### `GameStartedMessage`

Sent when the admin starts the game. Orders placed before this message is received will be rejected.

```json
{
    "GameStartedMessage": "GameStarted"
}
```

---

### Error

Generic error string for malformed messages or authentication failures.

```json
{
    "Error": "TraderId does not match authenticated connection."
}
```

---

## Minimal Python Example

```python
import asyncio
import websockets
import json

TRADER_ID = "trader42"
PASSWORD = ["0", "0", "4", "2"]
URI = "ws://localhost:8080/orders/ws"

async def main():
    async with websockets.connect(
        URI,
        subprotocols=[f"{TRADER_ID}|{''.join(PASSWORD)}"],
    ) as ws:
        # Handle initial account info and game state
        account_info = json.loads(await ws.recv())
        game_state   = json.loads(await ws.recv())

        # Wait for game to start
        async for raw in ws:
            msg = json.loads(raw)

            if "GameStartedMessage" in msg:
                print("Game started — placing order")
                await ws.send(json.dumps({
                    "MessageType": "OrderRequest",
                    "OrderType": "Buy",
                    "Amount": 5,
                    "Price": 40,
                    "Symbol": "TT",
                    "TraderId": TRADER_ID,
                    "Password": PASSWORD,
                }))

            elif "OrderConfirmMessage" in msg:
                order_id = msg["OrderConfirmMessage"]["order_info"]["order_id"]
                print(f"Order confirmed: id={order_id}")

            elif "OrderFillMessage" in msg:
                fill = msg["OrderFillMessage"]
                print(f"Filled {fill['amount_filled']} @ {fill['price']}")

            elif "OrderPlaceErrorMessage" in msg:
                print(f"Order rejected: {msg['OrderPlaceErrorMessage']['error_details']}")

            elif "TradeOccurredMessage" in msg:
                t = msg["TradeOccurredMessage"]
                print(f"Market trade: {t['amount']} {t['symbol']} @ {t['price']}")

asyncio.run(main())
```

---

## Keeping State in Sync

Rather than polling `GameStateRequest`, track changes in real time using the broadcast messages:

| Event | Effect on book |
|---|---|
| `NewRestingOrderMessage` | Add `amount` to `buy_side[price]` or `sell_side[price]` |
| `TradeOccurredMessage` | Subtract `amount` from the `resting_side` at `price`; remove key if volume hits 0 |
| `CancelOccurredMessage` | Subtract `amount` from the relevant side at `price`; remove key if volume hits 0 |

Your local book starts from the `GameState` snapshot received on connection and stays current by applying these three event types as they arrive.
