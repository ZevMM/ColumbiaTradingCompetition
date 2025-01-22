import asyncio
import os
import websockets
import json
import argparse
import numpy as np
import threading
import time
import random


files = {
    #"JJS" : [None, 15, "flat", 400],
    #"TT" : "TT_data",
    #"TS" : "TS_data",
    "AD" : "AD_data",
}

def bot_lookup(name):
    match name:
        case "JJS":
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


async def price_bot(key):
    fname = files[key]
    rng = bot_lookup(key)
    file = open(fname, '+a')

    while(True):
        await asyncio.sleep(15)
        file.write(str(rng.pull()) + "\n")
        file.flush()


async def main():
        tasks = []
        
        for key in files:
            task = asyncio.create_task(price_bot(key))
            tasks.append(task)
        
        await asyncio.wait(tasks)


if __name__ == "__main__":
    asyncio.run(main())