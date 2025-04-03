import time
import os
import websockets
import json
import numpy as np
import random
import asyncio

websocket_uri = "ws://localhost:8080/orders/ws"

'''
("TT", "./data/TT_demo"),
("TS", "./data/TS_demo"),
("AD", "./data/AD_demo"),
'''

assets = [
    ("AD", "./data/AD_demo"),
    ("TT", "./data/TT_demo"),
    ("TS", "./data/TS_demo"),
]

async def place_order(ws, price, symbol):

    
    jsonreq = {
        'MessageType' : 'AccountInfoRequest',
        'TraderId': "Price_Enforcer",
        'Password': list("penf")
    }
    await ws.send(json.dumps(jsonreq))
    

    await asyncio.sleep(1.5)

    print(f"Placing orders for {symbol}")
    for i in range(price - 7, price - 2):
        jsonreq = {
            'MessageType' : "OrderRequest",
            'OrderType': "Sell",
            'Amount': 90,
            'Price': max(0, i),
            'Symbol': symbol,
            'TraderId': "Price_Enforcer",
            'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))

    for i in range(price + 8, price + 13):
        jsonreq = {
            'MessageType' : "OrderRequest",
            'OrderType': "Sell",
            'Amount': 90,
            'Price': min(100, i),
            'Symbol': symbol,
            'TraderId': "Price_Enforcer",
            'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))
        
    await asyncio.sleep(1.5)

    for i in range(price - 12, price - 7):
        jsonreq = {
            'MessageType' : "OrderRequest",
            'OrderType': "Buy",
            'Amount': 90,
            'Price': max(0, i),
            'Symbol': symbol,
            'TraderId': "Price_Enforcer",
            'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))
    
    for i in range(price + 3, price + 8):
        jsonreq = {
                'MessageType' : "OrderRequest",
                'OrderType': "Buy",
                'Amount': 90,
                'Price': min(100, i),
                'Symbol': symbol,
                'TraderId': "Price_Enforcer",
                'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))


class from_file:
    def __init__(self, fname):
        self.file = open(fname, 'r')
    def pull(self):
        line = self.file.readline()
        if not line:
            self.file.seek(0)
            line = self.file.readline()
        return int(float(line.strip()))
    


async def price_bot(key, fname, ws, i):
    rng = from_file(fname)

    await asyncio.sleep(15 * (i + 1))

    while(True):
        await place_order(ws, rng.pull(), key)
        await asyncio.sleep(42.5)


async def main():
    async with websockets.connect(websocket_uri, subprotocols=["Price_Enforcer|penf"]) as ws:
        tasks = []
        for i, (key, fname) in enumerate(assets):
            task = asyncio.create_task(price_bot(key, fname, ws, i))
            tasks.append(task)
        
        #seems like waiting for threads to finish blocks the ws from
        #responding to ping messages.
        cur = 0
        first = True
        while(1):

            msg = await ws.recv()
            msg = list(json.loads(msg).items())[0]
            type, body = msg
            if type == "AccountInfo":
                if first:
                    first = False
                    continue
                symbol = assets[cur][0]
                print("Clearing orders for", symbol)
                for order in body["active_orders"]:
                    if order["symbol"] == symbol:
                        jsonreq = {
                            'MessageType' : "CancelRequest",
                            'OrderId': int(order["order_id"]),
                            'Side': order["order_type"],
                            'Price': order["price"],
                            'Symbol': symbol,
                            'TraderId': "Price_Enforcer",
                            'Password': list("penf")
                        }
                        await ws.send(json.dumps(jsonreq))
                cur = (cur + 1) % len(assets)


if __name__ == "__main__":
    asyncio.run(main())