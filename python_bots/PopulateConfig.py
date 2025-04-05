import json
import pandas as pd

max_price_cents = 100
start_cents_balance = 100000
start_asset_balance = 100000

accounts = [{"trader_id": "Price_Enforcer", "password": "penf"}, {"trader_id": "zev", "password": "0000"}]

jsonout= { "max_price_cents": max_price_cents,
          "start_asset_balance": start_asset_balance,
          "start_cents_balance" : start_cents_balance,
        "assets": [
        {
            "symbol": "AD",
            "long_name": "Average decible reading",
            "max_price_cents": 50
        },
        {
            "symbol": "TS",
            "long_name": "Times Square Webcam Brightness",
            "max_price_cents": 50
        },
        {
            "symbol": "TT",
            "long_name": "Average wait for 1 train at 116th (closest 8 times)",
            "max_price_cents": 50
        }
    ],
}

df = pd.read_csv("emails.csv", header=None)
for index, email in enumerate(df[0], start=1):
    
    trader_id = ''.join(e if e.isalnum() else "_" for e in email.split("@")[0]) 
    password = str(index).zfill(4)

    accounts.append({"trader_id": trader_id, "password": password})


jsonout["accounts"] = accounts
engine_conf = open("../matching-engine/config.json", "w")
engine_conf.write(json.dumps(jsonout))