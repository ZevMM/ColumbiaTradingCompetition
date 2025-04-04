import asyncio
import websockets

#move the normalization into this file (the running average/arctan/etc...)

websocket_uri = "wss://trading-competition-148005249496.us-east4.run.app/market_data/ws"

async def main():
    async with websockets.connect(websocket_uri) as ws:

        #with open("datastream", "w+") as f:

            while(1):
                msg = await ws.recv()
                print(msg)
                #f.write(msg + "\n")

if __name__ == "__main__":
    asyncio.run(main())