#!/usr/bin/env python3
"""
Robust market-making algo for the Columbia Trading Competition.

Strategy:
  - Maintains a local copy of the order book by applying broadcast messages.
  - Estimates a "fair value" for each asset using an EWMA over recent trade prices,
    seeded by the mid-price of the resting book.
  - Quotes a tight two-sided spread around fair value, biased by current inventory
    (lean against your position so you don't accumulate one-sided risk).
  - Cancels stale quotes when fair value moves more than `requote_threshold`.
  - Caps total order requests/second well under the server's rate limit.
  - Auto-reconnects with exponential backoff and resyncs state on reconnect.

Usage:
  pip install websockets
  python algo-trader.py --uid trader42 --pwd 0042 --url wss://exchange.columbia.trade

CLI flags (all optional except uid/pwd):
  --url          WebSocket base URL (default: ws://localhost:8080)
  --uid          Trader ID
  --pwd          4-char password
  --max-pos      Max absolute position per asset (default: 50)
  --quote-size   Size per quote (default: 5)
  --spread       Half-spread in cents around fair value (default: 2)
  --interval     Seconds between strategy ticks (default: 1.0)
  --log-level    DEBUG / INFO / WARNING (default: INFO)
"""

import argparse
import asyncio
import json
import logging
import random
import time
from collections import defaultdict, deque
from typing import Optional

import websockets

# ── CLI ──────────────────────────────────────────────────────────────────────

def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--url", default="ws://localhost:8080")
    p.add_argument("--uid", required=True)
    p.add_argument("--pwd", required=True)
    p.add_argument("--max-pos", type=int, default=50)
    p.add_argument("--quote-size", type=int, default=5)
    p.add_argument("--spread", type=int, default=2)
    p.add_argument("--interval", type=float, default=1.0)
    p.add_argument("--log-level", default="INFO")
    return p.parse_args()


# ── Local state ──────────────────────────────────────────────────────────────

class BookState:
    """Local mirror of the per-symbol order book."""
    def __init__(self):
        self.buy_side: dict[int, int] = {}   # price -> total volume
        self.sell_side: dict[int, int] = {}
        self.last_trade_prices: deque[int] = deque(maxlen=20)
        self.fair_value: Optional[float] = None  # EWMA of recent fills

    def best_bid(self) -> Optional[int]:
        return max(self.buy_side.keys()) if self.buy_side else None

    def best_ask(self) -> Optional[int]:
        return min(self.sell_side.keys()) if self.sell_side else None

    def mid(self) -> Optional[float]:
        b, a = self.best_bid(), self.best_ask()
        if b is not None and a is not None:
            return (b + a) / 2
        return None

    def update_fair_value(self, alpha: float = 0.3):
        """Recalculate fair value: EWMA of recent trades, fall back to mid."""
        if self.last_trade_prices:
            ewma = float(self.last_trade_prices[0])
            for p in list(self.last_trade_prices)[1:]:
                ewma = alpha * p + (1 - alpha) * ewma
            self.fair_value = ewma
        elif self.mid() is not None:
            self.fair_value = self.mid()


class TraderState:
    def __init__(self, uid: str, pwd: str):
        self.uid = uid
        self.pwd_array = list(pwd)
        self.cents_balance: int = 0
        self.net_cents_balance: int = 0
        self.asset_balances: dict[str, int] = defaultdict(int)
        self.net_asset_balances: dict[str, int] = defaultdict(int)
        # Track our open orders by id: {order_id: {"symbol", "side", "price", "amount"}}
        self.open_orders: dict[int, dict] = {}
        self.books: dict[str, BookState] = defaultdict(BookState)
        self.game_started = False


# ── Rate limiting (token-bucket-ish) ─────────────────────────────────────────

class RateLimiter:
    """Cap outgoing requests under the server's 10/s limit. Leave headroom."""
    def __init__(self, max_per_sec: int = 8):
        self.window = deque()
        self.max = max_per_sec

    async def acquire(self):
        while True:
            now = time.monotonic()
            # Drop entries older than 1 second
            while self.window and now - self.window[0] > 1.0:
                self.window.popleft()
            if len(self.window) < self.max:
                self.window.append(now)
                return
            sleep_for = max(0.0, 1.0 - (now - self.window[0])) + 0.01
            await asyncio.sleep(sleep_for)


# ── Bot ──────────────────────────────────────────────────────────────────────

class AlgoBot:
    def __init__(self, args):
        self.args = args
        self.state = TraderState(args.uid, args.pwd)
        self.limiter = RateLimiter(max_per_sec=8)
        self.ws: Optional[websockets.WebSocketClientProtocol] = None
        self.log = logging.getLogger("algo")
        self.send_queue: asyncio.Queue = asyncio.Queue()

    # ── Sending ──────────────────────────────────────────────────────────────

    async def send(self, msg: dict):
        await self.send_queue.put(msg)

    async def _sender_loop(self):
        """Single sender task. Rate-limits and writes to the socket."""
        while True:
            msg = await self.send_queue.get()
            if msg.get("MessageType") in ("OrderRequest", "CancelRequest"):
                await self.limiter.acquire()
            try:
                await self.ws.send(json.dumps(msg))
            except Exception as e:
                self.log.warning(f"send failed: {e}")
                return  # Let outer loop reconnect

    async def place_order(self, side: str, symbol: str, price: int, amount: int):
        await self.send({
            "MessageType": "OrderRequest",
            "OrderType": side,
            "Amount": amount,
            "Price": price,
            "Symbol": symbol,
            "TraderId": self.state.uid,
            "Password": self.state.pwd_array,
        })

    async def cancel_order(self, order_id: int, side: str, symbol: str, price: int):
        await self.send({
            "MessageType": "CancelRequest",
            "OrderId": int(order_id),
            "TraderId": self.state.uid,
            "Price": int(price),
            "Symbol": symbol,
            "Side": side,
            "Password": self.state.pwd_array,
        })

    # ── Receiving / state mirror ─────────────────────────────────────────────

    def _apply_account_info(self, body: dict):
        self.state.cents_balance = body["cents_balance"]
        self.state.net_cents_balance = body["net_cents_balance"]
        self.state.asset_balances = defaultdict(int, body.get("asset_balances", {}))
        self.state.net_asset_balances = defaultdict(int, body.get("net_asset_balances", {}))
        # Rebuild open_orders from authoritative server state
        self.state.open_orders.clear()
        for o in body.get("active_orders", []):
            self.state.open_orders[o["order_id"]] = {
                "symbol": o["symbol"],
                "side": o["order_type"],
                "price": o["price"],
                "amount": o["amount"],
            }
        self.log.info(
            f"AccountInfo: cash={self.state.cents_balance} "
            f"net={self.state.net_cents_balance} "
            f"pos={dict(self.state.asset_balances)} "
            f"open_orders={len(self.state.open_orders)}"
        )

    def _apply_game_state(self, body: dict):
        for symbol, snap in body.items():
            book = self.state.books[symbol]
            book.buy_side = {int(p): v for p, v in snap.get("buy_side", {}).items()}
            book.sell_side = {int(p): v for p, v in snap.get("sell_side", {}).items()}
            for entry in snap.get("price_history", [])[-20:]:
                book.last_trade_prices.append(int(entry[1]))
            book.update_fair_value()

    def _apply_trade(self, body: dict):
        symbol = body["symbol"]
        price = int(body["price"])
        amt = int(body["amount"])
        resting = body["resting_side"]
        book = self.state.books[symbol]
        side_dict = book.buy_side if resting == "Buy" else book.sell_side
        if price in side_dict:
            side_dict[price] -= amt
            if side_dict[price] <= 0:
                del side_dict[price]
        book.last_trade_prices.append(price)
        book.update_fair_value()

    def _apply_new_resting(self, body: dict):
        symbol = body["symbol"]
        price = int(body["price"])
        amt = int(body["amount"])
        side = body["side"]
        book = self.state.books[symbol]
        side_dict = book.buy_side if side == "Buy" else book.sell_side
        side_dict[price] = side_dict.get(price, 0) + amt

    def _apply_cancel_occurred(self, body: dict):
        symbol = body["symbol"]
        price = int(body["price"])
        amt = int(body["amount"])
        side = body["side"]
        book = self.state.books[symbol]
        side_dict = book.buy_side if side == "Buy" else book.sell_side
        if price in side_dict:
            side_dict[price] -= amt
            if side_dict[price] <= 0:
                del side_dict[price]

    def _apply_order_confirm(self, body: dict):
        info = body["order_info"]
        self.state.open_orders[info["order_id"]] = {
            "symbol": info["symbol"],
            "side": info["order_type"],
            "price": info["price"],
            "amount": info["amount"],
        }

    def _apply_fill(self, body: dict):
        oid = body["order_id"]
        amt = int(body["amount_filled"])
        price = int(body["price"])
        order = self.state.open_orders.get(oid)
        if order is None:
            self.log.debug(f"fill for unknown order {oid}")
            return
        symbol = order["symbol"]
        side = order["side"]
        if side == "Buy":
            self.state.cents_balance -= price * amt
            self.state.asset_balances[symbol] = self.state.asset_balances.get(symbol, 0) + amt
            self.state.net_asset_balances[symbol] = self.state.net_asset_balances.get(symbol, 0) + amt
        else:
            self.state.cents_balance += price * amt
            self.state.net_cents_balance += price * amt
            self.state.asset_balances[symbol] = self.state.asset_balances.get(symbol, 0) - amt
        order["amount"] -= amt
        if order["amount"] <= 0:
            del self.state.open_orders[oid]
        self.log.info(
            f"FILL {side:4s} {amt:>3} {symbol} @ {price}  "
            f"pos[{symbol}]={self.state.asset_balances.get(symbol, 0)}  "
            f"cash={self.state.cents_balance}"
        )

    def _apply_cancel_confirm(self, body: dict):
        info = body["order_info"]
        oid = info["order_id"]
        order = self.state.open_orders.pop(oid, None)
        if order is None:
            return
        if order["side"] == "Buy":
            self.state.net_cents_balance += order["price"] * order["amount"]
        else:
            sym = order["symbol"]
            self.state.net_asset_balances[sym] = self.state.net_asset_balances.get(sym, 0) + order["amount"]

    async def _receiver_loop(self):
        async for raw in self.ws:
            try:
                msg = json.loads(raw)
            except Exception:
                continue
            for msg_type, body in msg.items():
                if msg_type == "AccountInfo":
                    self._apply_account_info(body)
                elif msg_type == "GameState":
                    self._apply_game_state(body)
                elif msg_type == "GameStartedMessage":
                    self.state.game_started = True
                    self.log.info("GAME STARTED")
                elif msg_type == "TradeOccurredMessage":
                    self._apply_trade(body)
                elif msg_type == "NewRestingOrderMessage":
                    self._apply_new_resting(body)
                elif msg_type == "CancelOccurredMessage":
                    self._apply_cancel_occurred(body)
                elif msg_type == "OrderConfirmMessage":
                    self._apply_order_confirm(body)
                elif msg_type == "OrderFillMessage":
                    self._apply_fill(body)
                elif msg_type == "CancelConfirmMessage":
                    self._apply_cancel_confirm(body)
                elif msg_type == "OrderPlaceErrorMessage":
                    err = body.get("error_details", "")
                    if "Rate limit" not in err:
                        self.log.warning(f"order rejected: {err}")
                elif msg_type == "CancelErrorMessage":
                    pass  # Quiet — these happen if a fill races a cancel
                elif msg_type == "Error":
                    self.log.error(f"server error: {body}")
                    if isinstance(body, str) and "another session" in body.lower():
                        raise RuntimeError("kicked")

    # ── Strategy ─────────────────────────────────────────────────────────────

    async def _strategy_loop(self):
        await asyncio.sleep(2)  # Let the initial state settle
        last_summary = time.monotonic()
        while True:
            try:
                if self.state.game_started:
                    await self._tick()
                if time.monotonic() - last_summary > 15:
                    self._print_summary()
                    last_summary = time.monotonic()
            except Exception as e:
                self.log.exception(f"strategy tick error: {e}")
            await asyncio.sleep(self.args.interval)

    def _print_summary(self):
        positions = {s: v for s, v in self.state.asset_balances.items() if v != 0}
        urpl = 0.0
        for sym, qty in positions.items():
            book = self.state.books.get(sym)
            mid = book.mid() if book else None
            if mid is not None:
                urpl += qty * mid
        net_value = self.state.cents_balance + urpl
        self.log.info(
            f"== SUMMARY  cash={self.state.cents_balance}  "
            f"pos={positions or '{}'}  "
            f"urpl={urpl:.0f}  net={net_value:.0f}  "
            f"open_orders={len(self.state.open_orders)} =="
        )

    async def _tick(self):
        for symbol in list(self.state.books.keys()):
            await self._tick_symbol(symbol)

    async def _tick_symbol(self, symbol: str):
        book = self.state.books[symbol]
        book.update_fair_value()
        fv = book.fair_value
        if fv is None:
            return

        position = self.state.asset_balances.get(symbol, 0)
        max_pos = self.args.max_pos
        spread = self.args.spread
        size = self.args.quote_size

        # Lean: when long, drop bid + ask. When short, raise bid + ask.
        # Lean strength scales with how close we are to max_pos.
        lean = -int(round(2 * spread * position / max(1, max_pos)))
        target_bid = int(round(fv - spread + lean))
        target_ask = int(round(fv + spread + lean))

        # Don't cross our own quotes
        if target_ask <= target_bid:
            target_ask = target_bid + 1

        # Cancel any of our orders for this symbol that are far from target.
        # "Far" = more than 1 cent from target on the same side.
        to_cancel = []
        keep_bid = False
        keep_ask = False
        for oid, o in list(self.state.open_orders.items()):
            if o["symbol"] != symbol:
                continue
            if o["side"] == "Buy":
                if abs(o["price"] - target_bid) > 1:
                    to_cancel.append(oid)
                else:
                    keep_bid = True
            else:
                if abs(o["price"] - target_ask) > 1:
                    to_cancel.append(oid)
                else:
                    keep_ask = True

        for oid in to_cancel:
            o = self.state.open_orders.get(oid)
            if o:
                await self.cancel_order(oid, o["side"], o["symbol"], o["price"])

        # Place new quotes if we don't already have one near target and
        # if we're not already maxed out on inventory in that direction.
        if not keep_bid and position < max_pos:
            cost = target_bid * size
            if self.state.net_cents_balance >= cost and target_bid > 0:
                await self.place_order("Buy", symbol, target_bid, size)

        if not keep_ask and position > -max_pos:
            net_assets = self.state.net_asset_balances.get(symbol, 0)
            if net_assets >= size:
                await self.place_order("Sell", symbol, target_ask, size)

        self.log.debug(
            f"{symbol}: fv={fv:.1f} pos={position} bid={target_bid} ask={target_ask} "
            f"open={sum(1 for o in self.state.open_orders.values() if o['symbol']==symbol)}"
        )

    # ── Main connection loop ─────────────────────────────────────────────────

    async def run(self):
        backoff = 2
        while True:
            try:
                await self._connect_and_run()
                backoff = 2
            except RuntimeError as e:
                self.log.error(f"fatal: {e}")
                return
            except Exception as e:
                self.log.warning(f"connection error: {e}")
            wait = backoff + random.uniform(0, backoff * 0.3)
            self.log.info(f"reconnecting in {wait:.1f}s")
            await asyncio.sleep(wait)
            backoff = min(backoff * 1.5, 30)

    async def _connect_and_run(self):
        url = f"{self.args.url}/orders/ws"
        protocol = f"{self.state.uid}|{self.args.pwd}"
        self.log.info(f"connecting to {url} as {self.state.uid}")
        async with websockets.connect(url, subprotocols=[protocol], ping_interval=20) as ws:
            self.ws = ws
            self.log.info("connected")
            sender_task = asyncio.create_task(self._sender_loop())
            strategy_task = asyncio.create_task(self._strategy_loop())
            try:
                await self._receiver_loop()
            finally:
                sender_task.cancel()
                strategy_task.cancel()
                # Drain queued sends so we don't replay stale orders
                while not self.send_queue.empty():
                    self.send_queue.get_nowait()


async def main():
    args = parse_args()
    logging.basicConfig(
        level=getattr(logging, args.log_level.upper(), logging.INFO),
        format="%(asctime)s [%(levelname)s] %(message)s",
    )
    bot = AlgoBot(args)
    await bot.run()


if __name__ == "__main__":
    asyncio.run(main())
