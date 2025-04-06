import json
import pandas as pd

max_price_cents = 101
start_cents_balance = 10000
start_asset_balance = 100

accounts = [{"trader_id": "Price_Enforcer", "password": "penf"}, {"trader_id": "zev", "password": "0000"}, {"trader_id": "test1", "password": "00t1"}, {"trader_id": "test2", "password": "00t2"}, {"trader_id": "test3", "password": "00t3"}, {"trader_id": "test4", "password": "00t4"}, {"trader_id": "test5", "password": "00t5"}]

jsonout= { "max_price_cents": max_price_cents,
          "start_asset_balance": start_asset_balance,
          "start_cents_balance" : start_cents_balance,
        "assets": [
        {
            "symbol": "AD",
        },
        {
            "symbol": "TS",
        },
        {
            "symbol": "TT",
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