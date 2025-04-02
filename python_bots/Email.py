
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

with smtplib.SMTP(smtp_server, smtp_port) as server:
    server.starttls()
    server.login(sender_email, sender_password) 

    df = pd.read_csv("emails.csv", header=None)
    for index, email in enumerate(df[0], start=1):
        
        message = MIMEMultipart()
        message["From"] = sender_email
        message["Subject"] = subject
        message["To"] = email
        trader_id = ''.join(e if e.isalnum() else "_" for e in email.split("@")[0]) 
        password = str(index).zfill(4)

        body = f'''
        Web Client: https://zevmm.github.io/ColumbiaTradingCompetition/ \n
        \n
        Datastream: https://trading-competition-148005249496.us-central1.run.app/market_data/ws \n
        ================== \n
        \n
        Trader Id: {trader_id} \n
        Password: {password}
        '''
        message.attach(MIMEText(body, "plain"))
        server.sendmail(sender_email, email, message.as_string())
        print(f"Email sent to {email}")