import time
import websocket
import json
import argparse
import random
import schedule
import math

from scipy.interpolate import interp1d

# NEED TO SET UP  FOR TESTING
# sudo ifconfig lo0 alias 127.16.123.1 (columbia a)
# sudo ifconfig lo0 alias 127.16.123.2 (columbia b)

parser = argparse.ArgumentParser()

parser.add_argument('--ip', type=str, required=True)
parser.add_argument('--id', type=str, required=True)
parser.add_argument('--password', type=str, required=True)

args = parser.parse_args()

ws = websocket.WebSocket()

ws.connect(f"ws://{args.ip}:4000/orders/ws")


jsonreq = {
    'OrderType': "Buy",
    'Amount': 1,
    'Price': 9,
    'Symbol': "AAPL",
    'TraderId': f"{args.id}",
}

# ws.send(json.dumps(jsonreq))
# ws.send("Hello")
# time.sleep(20)

m = interp1d([-1, 1], [1, 10])
off = random.randint(-3,3)

def trade_rand():
    global flip
    # symbol = random.choice(["AAPL"])
    symbol = "AAPL"
    # price = int(m(math.sin(time.time()+off)))
    price = 1
    # side = random.choice(["Sell", "Buy"])
    side = "Buy"
    # if flip:
    #     flip = False
    #     side="Buy"
    # else:
    #     side="Sell"
    #     flip = True
    # amount = random.randint(1, 10)
    jsonreq = {
        'OrderType': side,
        'Amount': 1,
        'Price': price,
        'Symbol': symbol,
        'TraderId': f"{args.id}",
        'Password': f"{args.password.split('')}"
    }

    ws.send(json.dumps(jsonreq))

# schedule.every(1).seconds.do(trade_rand)
# flip = False
# for i in range(1000):
#     trade_rand()
flip = False
time_start = time.time()
time_old = time.time()
time_old2 = time.time()
msgs = 0

# trade_rand()
# while True: 
#     print(ws.recv())
while True:    
    if(time.time() - time_old > 3):
        ws.ping()
        print(time.time() - time_old)
        time_old = time.time()
        
        # goal should be ~1M/S (i.e. <1us per order server side)
    # this runs @6700 req/sec, server is fine after removing sync arbiter issue.  
    if(time.time() - time_old2 > 0.1):
        trade_rand()
        time_old2 = time.time()
        el = time_old2 - time_start
        msgs += 1
        print("msg/sec:", msgs/el)
        print("total msgs sent: ", msgs)
    print(ws.recv())
    # time.sleep(0.1)    
