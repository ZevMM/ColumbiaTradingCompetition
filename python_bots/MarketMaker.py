import asyncio
import os
import websockets
import json
import argparse
import numpy as np
import threading
import time
import random

#move the normalization into this file (the running average/arctan/etc...)

websocket_uri = "wss://trading-competition-148005249496.us-east4.run.app/orders/ws"

#{symbol: [filename, avg frequency (s), dist, amt (total shares?)]
settings = {
    #"JJs" : [None, 15, "flat", 400],
    "TT" : ["./data/TT_data", 15, "flat", 400],
    "TS" : ["./data/TS_data", 15, "flat", 400],
    "AD" : ["./data/AD_data", 15, "flat", 400],
}

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

# todo: implement bimodal, delta, smile and see what is the most fun
def gen_dist(dist, amt):
    match dist:
        case "flat":
            return [amt // 100] * 100
        case "normal":
            indices = np.arange(100)
            #15 is std dev, can play around with it
            normal_values = np.exp(-(indices - 50) ** 2 / (2 * 15 ** 2))
            normal_values *=  (amt / normal_values.sum())
            return normal_values.astype(int)

async def place_order(ws, price, dist, amt, symbol):
    #amts = gen_dist(dist, amt)

    for i in range(0, 15):
        jsonreq = {
            'MessageType' : "OrderRequest",
            'OrderType': "Sell",
            'Amount': random.randint(0,10),
            'Price': min(int(price) - 3 + i, 49),
            'Symbol': symbol,
            'TraderId': "Price_Enforcer",
            'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))
        await asyncio.sleep(0.25)

            
    for i in range(0, 15):
        jsonreq = {
                'MessageType' : "OrderRequest",
                'OrderType': "Buy",
                'Amount': random.randint(0,10),
                'Price': max(int(price) + i - 10, 0),
                'Symbol': symbol,
                'TraderId': "Price_Enforcer",
                'Password': list("penf")
        }
        await ws.send(json.dumps(jsonreq))
        await asyncio.sleep(0.25)

class from_file:
    #should the file store the time stamps for each entry?
    def __init__(self, fname):
        self.file = open(fname, 'rb')
    def pull(self):
        line = self.file.readline()
        if not line:
            print("here")
            self.file.seek(0)
            line = self.file.readline()
        return float(line.strip())

async def price_bot(key, ws):
    fname, interval, dist, amt = settings[key]
    
    rng = from_file(fname) if fname else bot_lookup(key)

    while(True):
        await asyncio.sleep(abs(random.gauss(interval, interval / 3)))
        await place_order(ws, rng.pull(), dist, amt, key)


async def main():
    async with websockets.connect(websocket_uri, subprotocols=["Price_Enforcer"]) as ws:
        tasks = []
        
        for key in settings:
            task = asyncio.create_task(price_bot(key, ws))
            tasks.append(task)
        
        #seems like waiting for threads to finish blocks the ws from
        #responding to ping messages.
        while(1):
            msg = await ws.recv()
            print(msg)


if __name__ == "__main__":
    asyncio.run(main())