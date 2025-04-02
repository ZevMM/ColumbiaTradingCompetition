import asyncio
import websockets

#move the normalization into this file (the running average/arctan/etc...)

websocket_uri = "ws://localhost:8080/market_data/ws"

async def main():
    async with websockets.connect(websocket_uri) as ws:

        with open("datastream", "w+") as f:

            while(1):
                msg = await ws.recv()
                print(msg)
                f.write(msg + "\n")

if __name__ == "__main__":
    asyncio.run(main())