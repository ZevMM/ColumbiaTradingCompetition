
import smtplib
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
import random
import string
import json

import pandas as pd

file = open("./sender_password")

sender_email = "zevmcmanusmendelowitz@gmail.com"
sender_password = file.readline()
max_price_cents = 100
start_asset_balance = 100

smtp_server = "smtp.gmail.com"
smtp_port = 587

subject = "Competition Details"

message = MIMEMultipart()
message["From"] = sender_email
message["Subject"] = subject

start_cents_balance = 10000
start_asset_balance = 100

accounts = [{"trader_id": "Price_Enforcer", "password": "penf"}, {"trader_id": "zev", "password": "0000"}, {"trader_id": "ryan", "password": "6767"}]

jsonout= {
          "order_rate_limit_per_second": 10,
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

with smtplib.SMTP(smtp_server, smtp_port) as server:
    server.starttls()
    server.login(sender_email, sender_password) 

    df = pd.read_csv("emails.csv", header=None)
    for index, email in enumerate(df[0], start=1):
        
        message = MIMEMultipart()
        message["From"] = sender_email
        message["Subject"] = subject
        message["To"] = email
        trader_id = f"trader{index}"
        password = ''.join(random.choices(string.ascii_letters + string.digits, k=4))
        accounts.append({"trader_id": trader_id, "password": password})
        body = f'''
        Web Client: https://exchange.columbia.trade/ \n
        \n
        API Endpoint: wss://exchange.columbia.trade/orders/ws \n
        \n
        Datastream: wss://exchange.columbia.trade/market_data/ws \n
        ================== \n
        \n
        Trader Id: {trader_id} \n
        Password: {password}
        '''
        message.attach(MIMEText(body, "plain"))
        server.sendmail(sender_email, email, message.as_string())
        print(f"Email sent to {email}")

    jsonout["accounts"] = accounts
    with open("../matching-engine/config.json", "w") as engine_conf:
        engine_conf.write(json.dumps(jsonout))