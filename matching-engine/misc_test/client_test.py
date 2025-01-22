import requests
import csv
# {\"Amount\":10,\"Price\":1,\"OrderType\":\"Buy\",\"TraderId\":1,\"Symbol\":\"AAPL\"}
with open('../src/test_orders.csv', newline='') as csvfile:
    spamreader = csv.reader(csvfile, delimiter=',', quotechar='|')
    _h = 0
    for row in spamreader: 
        if _h == 0:
            _h += 1
            continue       
        jsonreq = {
            'OrderType': row[0],
            'Amount': int(row[1]),
            'Price': int(row[2]),
            'Symbol': row[3],
            'TraderId': row[4],            
        }
        r = requests.post(url="http://127.0.0.1:3000/orders/addOrder", json=jsonreq)


# sending post request and saving response as response object

