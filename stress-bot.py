#!/usr/bin/env python3
"""
Stress-test bot — hammers the exchange with orders as fast as the rate limiter allows.
Run on multiple machines simultaneously to test from different IPs.

Usage:
    pip install websockets
    python stress-bot.py --url ws://localhost:8080 --traders 10
    python stress-bot.py --url wss://exchange.columbia.trade --traders 5 --duration 60

Flags:
    --url           WebSocket base URL (default: ws://localhost:8080)
    --traders       Number of concurrent traders (default: 10)
    --prefix        Trader ID prefix (default: stress). Use different prefixes per machine.
    --duration      Seconds to run (default: 120)
    --delay         Seconds between orders per trader (default: 0.15, just under 5/s limit)
"""

import argparse
import asyncio
import json
import random
import time

import websockets

ASSETS = ["AD", "TS", "TT"]


def parse_args():
    p = argparse.ArgumentParser(description="Exchange stress-test bot")
    p.add_argument("--url", default="ws://localhost:8080")
    p.add_argument("--traders", type=int, default=10)
    p.add_argument("--prefix", default="stress")
    p.add_argument("--duration", type=int, default=120)
    p.add_argument("--delay", type=float, default=0.15)
    return p.parse_args()


stats = {
    "connected": 0,
    "sent": 0,
    "confirmed": 0,
    "rejected": 0,
    "rate_limited": 0,
    "fills": 0,
    "errors": 0,
}
lock = asyncio.Lock()


async def inc(key, n=1):
    async with lock:
        stats[key] += n


async def run_trader(url, trader_id, password, duration, delay):
    uri = f"{url}/orders/ws"
    pw_array = list(password)

    try:
        async with websockets.connect(
            uri, subprotocols=[f"{trader_id}|{password}"]
        ) as ws:
            await inc("connected")
            print(f"[+] {trader_id} connected ({stats['connected']} total)")

            # Wait for initial AccountInfo + GameState
            await ws.recv()
            await ws.recv()

            end_time = time.time() + duration

            async def send_orders():
                while time.time() < end_time:
                    asset = random.choice(ASSETS)
                    side = random.choice(["Buy", "Sell"])
                    price = random.randint(20, 80)
                    amount = random.randint(1, 15)

                    msg = {
                        "MessageType": "OrderRequest",
                        "OrderType": side,
                        "Amount": amount,
                        "Price": price,
                        "Symbol": asset,
                        "TraderId": trader_id,
                        "Password": pw_array,
                    }
                    await ws.send(json.dumps(msg))
                    await inc("sent")

                    # Occasionally cancel a random order to vary message types
                    if random.random() < 0.1:
                        cancel = {
                            "MessageType": "CancelRequest",
                            "OrderId": random.randint(1, 100000),
                            "TraderId": trader_id,
                            "Price": price,
                            "Symbol": asset,
                            "Side": side,
                            "Password": pw_array,
                        }
                        await ws.send(json.dumps(cancel))
                        await inc("sent")

                    await asyncio.sleep(delay)

            async def recv_messages():
                try:
                    async for raw in ws:
                        msg = json.loads(raw)
                        if "OrderConfirmMessage" in msg:
                            await inc("confirmed")
                        elif "OrderPlaceErrorMessage" in msg:
                            err = msg["OrderPlaceErrorMessage"].get("error_details", "")
                            if "Rate limit" in err:
                                await inc("rate_limited")
                            else:
                                await inc("rejected")
                        elif "OrderFillMessage" in msg:
                            await inc("fills")
                        elif "Error" in msg:
                            await inc("errors")
                except websockets.exceptions.ConnectionClosed:
                    pass

            await asyncio.gather(send_orders(), recv_messages())

    except Exception as e:
        await inc("errors")
        print(f"[!] {trader_id}: {e}")


async def print_stats(duration):
    start = time.time()
    while time.time() - start < duration + 5:
        await asyncio.sleep(5)
        elapsed = time.time() - start
        async with lock:
            s = dict(stats)
        rate = s["sent"] / max(elapsed, 1)
        print(
            f"[{elapsed:5.0f}s] "
            f"conn={s['connected']}  "
            f"sent={s['sent']}  "
            f"confirmed={s['confirmed']}  "
            f"rejected={s['rejected']}  "
            f"rate_limited={s['rate_limited']}  "
            f"fills={s['fills']}  "
            f"errors={s['errors']}  "
            f"({rate:.1f} req/s)"
        )


async def main():
    args = parse_args()

    print(f"Stress test: {args.traders} traders -> {args.url}")
    print(f"  prefix={args.prefix}  delay={args.delay}s  duration={args.duration}s")
    print(f"  target rate per trader: {1/args.delay:.1f} req/s  (limit is 5/s)\n")

    tasks = [print_stats(args.duration)]

    for i in range(1, args.traders + 1):
        trader_id = f"{args.prefix}{i}"
        password = str(i).zfill(4)
        tasks.append(run_trader(args.url, trader_id, password, args.duration, args.delay))
        # Stagger connections slightly
        await asyncio.sleep(0.05)

    await asyncio.gather(*tasks)

    print("\n--- Final Stats ---")
    for k, v in stats.items():
        print(f"  {k:15s}: {v}")
    print()


if __name__ == "__main__":
    asyncio.run(main())
