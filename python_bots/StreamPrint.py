import asyncio
import websockets

websocket_uri = "ws://localhost/orders/ws"

async def main():
    async with websockets.connect(websocket_uri,
                                  subprotocols=["ryan|6767"]
                                  ) as ws:

        with open("datastream", "w+") as f:

            while(1):
                msg = await ws.recv()
                print(msg)
                f.write(msg + "\n")

if __name__ == "__main__":
    asyncio.run(main())