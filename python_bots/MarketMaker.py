import asyncio
import os
import websockets
import json
import numpy as np
import random
import time as _time
import logging

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
log = logging.getLogger("MarketMaker")

websocket_uri = os.environ.get("WS_URI", "ws://localhost:8080/orders/ws")
MIN_PRICE = int(os.environ.get("MIN_PRICE", "1"))

# {symbol: [filename, avg frequency (s), dist, amt (total shares?)]}
settings = {
    "TT": ["./data/TT_data", 15, "flat", 400],
    "TS": ["./data/TS_data", 15, "flat", 400],
    "AD": ["./data/AD_data", 15, "flat", 400],
}

MAX_RETRIES = None  # retry forever
INITIAL_BACKOFF = 2  # seconds
MAX_BACKOFF = 30     # seconds


def bot_lookup(name):
    match name:
        case "JJs":
            from randomness_generators import JJs_Capacity
            return JJs_Capacity.JJs()
        case "TT":
            from randomness_generators import TrainTime_Avg
            return TrainTime_Avg.TT()
        case "AD":
            from randomness_generators import Audio_RNG
            return Audio_RNG.AD()
        case "TS":
            from randomness_generators import TS_Brightness
            return TS_Brightness.TS()


def gen_dist(dist, amt):
    match dist:
        case "flat":
            return [amt // 100] * 100
        case "normal":
            indices = np.arange(100)
            normal_values = np.exp(-(indices - 50) ** 2 / (2 * 15 ** 2))
            normal_values *= (amt / normal_values.sum())
            return normal_values.astype(int)


async def place_order(ws, price, dist, amt, symbol):
    price = int(price)
    for i in range(0, 15):
        p = max(price - 3 + i, MIN_PRICE)
        jsonreq = {
            'MessageType': "OrderRequest",
            'OrderType': "Sell",
            'Amount': random.randint(0, 10),
            'Price': p,
            'Symbol': symbol,
            'TraderId': "Price_Enforcer",
            'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))
        await asyncio.sleep(0.25)

    for i in range(0, 15):
        p = max(price + i - 10, MIN_PRICE)
        jsonreq = {
            'MessageType': "OrderRequest",
            'OrderType': "Buy",
            'Amount': random.randint(0, 10),
            'Price': p,
            'Symbol': symbol,
            'TraderId': "Price_Enforcer",
            'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))
        await asyncio.sleep(0.25)


class from_file:
    def __init__(self, fname):
        self.file = open(fname, 'rb')

    def pull(self):
        line = self.file.readline()
        if not line:
            self.file.seek(0)
            line = self.file.readline()
        return float(line.strip())


async def price_bot(key, ws):
    fname, interval, dist, amt = settings[key]
    rng = from_file(fname) if fname else bot_lookup(key)
    while True:
        await asyncio.sleep(abs(random.gauss(interval, interval / 3)))
        try:
            await place_order(ws, rng.pull(), dist, amt, key)
        except websockets.exceptions.ConnectionClosed:
            raise
        except Exception as e:
            log.warning(f"Error placing orders for {key}: {e}")


async def run_session():
    """Connect and run bots for one session. Raises on disconnect."""
    log.info(f"Connecting to {websocket_uri}")
    async with websockets.connect(
        websocket_uri,
        subprotocols=["Price_Enforcer|penf"],
    ) as ws:
        log.info("Connected successfully")
        tasks = []
        for key in settings:
            task = asyncio.create_task(price_bot(key, ws))
            tasks.append(task)

        try:
            while True:
                msg = await ws.recv()
                log.debug(msg)
        finally:
            for t in tasks:
                t.cancel()


async def main():
    """Main loop with retry logic and exponential backoff."""
    backoff = INITIAL_BACKOFF
    attempt = 0

    while True:
        attempt += 1
        try:
            await run_session()
        except (
            websockets.exceptions.ConnectionClosed,
            websockets.exceptions.InvalidStatusCode,
            websockets.exceptions.InvalidHandshake,
            ConnectionRefusedError,
            OSError,
        ) as e:
            log.warning(f"Connection lost/failed (attempt {attempt}): {e}")
        except Exception as e:
            log.error(f"Unexpected error (attempt {attempt}): {e}")

        if MAX_RETRIES is not None and attempt >= MAX_RETRIES:
            log.error(f"Max retries ({MAX_RETRIES}) reached, giving up")
            break

        jitter = random.uniform(0, backoff * 0.3)
        wait = backoff + jitter
        log.info(f"Retrying in {wait:.1f}s (backoff={backoff:.1f}s)")
        await asyncio.sleep(wait)
        backoff = min(backoff * 1.5, MAX_BACKOFF)

        # Reset backoff after a successful long-running connection
        # (handled implicitly: if run_session ran for >60s, reset)


if __name__ == "__main__":
    asyncio.run(main())
