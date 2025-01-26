
import smtplib
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
import random
import string
import json


file = open("./sender_password")

def generate_random_string(length: int) -> str:
    characters = string.ascii_letters + string.digits
    random_string = ''.join(random.choice(characters) for _ in range(length))
    return random_string

sender_email = "zevmcmanusmendelowitz@gmail.com"
sender_password = file.readline()
max_price_cents = 100

trader_emails = ["ih2427@columbia.edu"]
server_addr = "ws://127.0.0.1:4000/orders/ws"

smtp_server = "smtp.gmail.com"
smtp_port = 587

subject = "Competition Details"

message = MIMEMultipart()
message["From"] = sender_email
message["Subject"] = subject

accounts = [{"trader_id": "Price_Enforcer", "password": "penf"}, {"trader_id": "zev", "password": "0000"}]

jsonout= { "max_price_cents": max_price_cents,
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

with smtplib.SMTP(smtp_server, smtp_port) as server:
    server.starttls()
    server.login(sender_email, sender_password) 

    for recipient_email in trader_emails:
        message = MIMEMultipart()
        message["From"] = sender_email
        message["Subject"] = subject
        message["To"] = recipient_email
        trader_id = ''.join(e if e.isalnum() else "_" for e in recipient_email.split("@")[0]) 
        password = generate_random_string(4)
        body = f'''
        https://zevmm.github.io/ColumbiaTradingCompetition/ \n
        https://drive.google.com/file/d/1xFue7NOyylHeSvFQFQteWvNKSYTOxxMa/view?usp=sharing \n 
        ================== \n
        Trader Id: {trader_id} \n
        Password: {password}
        '''
        accounts.append({"trader_id": trader_id, "password": password})
        message.attach(MIMEText(body, "plain"))

        server.sendmail(sender_email, recipient_email, message.as_string())
        print(f"Email sent to {recipient_email}")

jsonout["accounts"] = accounts
engine_conf = open("../matching-engine/config.json", "w")
engine_conf.write(json.dumps(jsonout))