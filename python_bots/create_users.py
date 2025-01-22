
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

trader_emails = ['anniem2716@gmail.com', 'sethmend60@gmail.com', 'zem2109@columbia.edu']
server_addr = "ws://127.0.0.1:4000/orders/ws"

smtp_server = "smtp.gmail.com"
smtp_port = 587

subject = "Competition Details"

message = MIMEMultipart()
message["From"] = sender_email
message["Subject"] = subject

jsonout = []

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
        http://192.168.1.73:5173/ExchangeClient/ \n 
        ================== \n
        Trader Id: {trader_id} \n
        Password: {password}
        '''
        jsonout.append({"trader_id": trader_id, "password": password})
        message.attach(MIMEText(body, "plain"))

        server.sendmail(sender_email, recipient_email, message.as_string())
        print(f"Email sent to {recipient_email}")

print(json.dumps(jsonout))