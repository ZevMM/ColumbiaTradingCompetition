import asyncio
import websockets
import json
async def echo(websocket):    
    jsonreq = {
            'OrderType': "Buy",
            'Amount': 3,
            'Price': 3,
            'Symbol': "AAPL",
            'TraderId': "Columbia_A",        
        }
    print("HELLO")
    await websocket.send(json.dump(jsonreq))
    # async for message in websocket:
        
async def main():
    async with websockets.serve(echo, "127.0.0.1", 8080):
        await asyncio.Future()  # run forever

asyncio.run(main())