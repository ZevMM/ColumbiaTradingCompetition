# Client to Server
## Limit Order Request Message
'OrderType': "Buy" | "Sell",
'Amount': int,
'Price': int,
'Symbol': string,
'TraderId': string,
'Password': 4 char array

## Cancel Order Request Message
'OrderType': "Buy" | "Sell",
'OrderID': string,
'Price': int,
'Symbol': string,
'Password': 4 char array

# Server to Client
## Fill Message
'OrderID': string,
'OrderType': "Buy" | "Sell",
'Price': int,
'Amount': int,
'Symbol': string

## Error Placing Order Message
'order_id': string,
'side': "Buy" | "Sell",
'price': int,
'amount': int,
'symbol': string
'error': string 

## Error Cancelling Order Message
'order_id': string,
'side': "Buy" | "Sell",
'symbol': string
'error': string 

## Orderbook Update Message
"level": int,
"side": "Buy" | "Sell",
"total_order_vol": int,
"symbol": string