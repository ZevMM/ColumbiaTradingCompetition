write full tests:
1. Successful Order + Cancel = Nothing (empty active_orders, 0 on total volume)
2. Order can be placed within price limits
3. Order cannot be placed outside of price limits
4. Shorting or over buying is not allowed
5. Passwords are enforced
6. All messages relevant to client state are preserved between connections (message queue etc.)