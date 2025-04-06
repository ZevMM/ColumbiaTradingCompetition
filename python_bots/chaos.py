import time
import websockets
import json
import asyncio

websocket_uri = "wss://trading-competition-148005249496.us-east4.run.app/orders/ws"

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
'''
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
'''

async def mytask(ws):
    for i in range(10010):
        jsonreq = {
            'MessageType' : "OrderRequest",
            'OrderType': "Sell",
            'Amount': 10,
            'Price': 0,
            'Symbol': "AD",
            'TraderId': "Price_Enforcer",
            'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))

async def main():
    async with websockets.connect(websocket_uri, subprotocols=["Price_Enforcer|penf"]) as ws:

        tasks = []
        task = asyncio.create_task(mytask(ws))
        tasks.append(task)

        while 1:
            msg = await ws.recv()
            print(msg)


if __name__ == "__main__":
    asyncio.run(main())