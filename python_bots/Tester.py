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

websocket_uri = "ws://localhost:8080/market_data/ws"

async def main():
    async with websockets.connect(websocket_uri) as ws:

        while(1):
            msg = await ws.recv()
            print(msg)

if __name__ == "__main__":
    asyncio.run(main())