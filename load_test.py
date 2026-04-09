#!/usr/bin/env python3
"""
Load test for the trading exchange.
Simulates N traders connecting via WebSocket and placing random orders.

Usage:
    pip install websockets
    python load_test.py                          # default: 60 traders, localhost
    python load_test.py --url ws://exchange.columbia.edu --traders 60
"""

import asyncio
import json
import random
import time
import argparse
from collections import defaultdict

try:
    import websockets
except ImportError:
    print("Install websockets: pip install websockets")
    exit(1)

SYMBOLS = ["AD", "TS", "TT"]
STATS = defaultdict(int)
ERRORS = []
LATENCIES = []
CONNECTED = 0


async def trader_bot(trader_id: str, password: str, base_url: str, duration: int):
    global CONNECTED
    orders_url = f"{base_url}/orders/ws"
    subprotocol = f"{trader_id}|{password}"

    try:
        async with websockets.connect(
            orders_url,
            subprotocols=[subprotocol],
            open_timeout=10,
        ) as ws:
            CONNECTED += 1
            STATS["connections"] += 1
            print(f"  [{trader_id}] connected ({CONNECTED} total)")

            end_time = time.time() + duration
            order_count = 0

            while time.time() < end_time:
                symbol = random.choice(SYMBOLS)
                side = random.choice(["Buy", "Sell"])
                price = random.randint(20, 80)
                amount = random.randint(1, 10)

                msg = json.dumps({
                    "MessageType": "OrderRequest",
                    "Price": price,
                    "TraderId": trader_id,
                    "OrderType": side,
                    "Amount": amount,
                    "Password": list(password),
                    "Symbol": symbol,
                })

                t0 = time.time()
                await ws.send(msg)

                try:
                    resp = await asyncio.wait_for(ws.recv(), timeout=5)
                    latency_ms = (time.time() - t0) * 1000
                    LATENCIES.append(latency_ms)

                    data = json.loads(resp)
                    if "OrderConfirmMessage" in str(data):
                        STATS["order_placed"] += 1
                    elif "OrderFillMessage" in str(data) or "TradeOccurredMessage" in str(data):
                        STATS["fills"] += 1
                    elif "Error" in str(data):
                        STATS["order_errors"] += 1
                    else:
                        STATS["other_messages"] += 1
                except asyncio.TimeoutError:
                    STATS["timeouts"] += 1

                order_count += 1

                # Drain any extra broadcast messages (fills, etc.)
                try:
                    while True:
                        extra = await asyncio.wait_for(ws.recv(), timeout=0.05)
                        STATS["broadcast_msgs"] += 1
                except asyncio.TimeoutError:
                    pass

                # Random delay: simulate human-ish trading (0.5-2s between orders)
                await asyncio.sleep(random.uniform(0.5, 2.0))

            CONNECTED -= 1

    except Exception as e:
        STATS["connection_errors"] += 1
        ERRORS.append(f"[{trader_id}] {type(e).__name__}: {e}")


async def market_data_listener(base_url: str, duration: int):
    """Simulates a few market data subscribers (like the frontend chart)."""
    url = f"{base_url}/market_data/ws"
    try:
        async with websockets.connect(url, open_timeout=10) as ws:
            STATS["md_connections"] += 1
            end_time = time.time() + duration
            while time.time() < end_time:
                try:
                    msg = await asyncio.wait_for(ws.recv(), timeout=5)
                    STATS["md_messages"] += 1
                except asyncio.TimeoutError:
                    pass
    except Exception as e:
        ERRORS.append(f"[market_data] {type(e).__name__}: {e}")


async def run_load_test(base_url: str, num_traders: int, duration: int):
    print(f"Load test: {num_traders} traders, {duration}s duration")
    print(f"Target: {base_url}\n")

    # Build trader list (trader1..traderN with 4-digit passwords)
    traders = []
    for i in range(1, num_traders + 1):
        trader_id = f"trader{i}"
        password = f"{i:04d}"
        traders.append((trader_id, password))

    # Stagger connections over 5 seconds to avoid a thundering herd
    tasks = []
    for i, (tid, pw) in enumerate(traders):
        delay = (i / num_traders) * 5  # spread over 5s
        tasks.append(delayed_start(delay, trader_bot(tid, pw, base_url, duration)))

    # Add a few market data listeners
    for _ in range(5):
        tasks.append(market_data_listener(base_url, duration))

    print("Ramping up connections...")
    t_start = time.time()
    await asyncio.gather(*tasks, return_exceptions=True)
    elapsed = time.time() - t_start

    # Report
    print("\n" + "=" * 50)
    print(f"LOAD TEST RESULTS ({elapsed:.1f}s elapsed)")
    print("=" * 50)
    for k, v in sorted(STATS.items()):
        print(f"  {k:25s}: {v}")

    if LATENCIES:
        LATENCIES.sort()
        print(f"\n  {'latency (min)':25s}: {LATENCIES[0]:.1f} ms")
        print(f"  {'latency (median)':25s}: {LATENCIES[len(LATENCIES)//2]:.1f} ms")
        print(f"  {'latency (p95)':25s}: {LATENCIES[int(len(LATENCIES)*0.95)]:.1f} ms")
        print(f"  {'latency (p99)':25s}: {LATENCIES[int(len(LATENCIES)*0.99)]:.1f} ms")
        print(f"  {'latency (max)':25s}: {LATENCIES[-1]:.1f} ms")
        print(f"  {'throughput':25s}: {len(LATENCIES)/elapsed:.1f} orders/sec")

    if ERRORS:
        print(f"\nErrors ({len(ERRORS)}):")
        for e in ERRORS[:20]:
            print(f"  {e}")


async def delayed_start(delay: float, coro):
    await asyncio.sleep(delay)
    return await coro


def main():
    parser = argparse.ArgumentParser(description="Exchange load tester")
    parser.add_argument("--url", default="ws://localhost:8080",
                        help="WebSocket base URL (default: ws://localhost:8080)")
    parser.add_argument("--traders", type=int, default=60,
                        help="Number of simulated traders (default: 60)")
    parser.add_argument("--duration", type=int, default=30,
                        help="Test duration in seconds (default: 30)")
    args = parser.parse_args()

    asyncio.run(run_load_test(args.url, args.traders, args.duration))


if __name__ == "__main__":
    main()
