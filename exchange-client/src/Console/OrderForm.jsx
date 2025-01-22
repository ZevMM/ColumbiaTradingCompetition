let placeOrder = (e, side, user, ws) => {
    e.preventDefault()
    const order = new FormData(e.target)

    const message = {
        MessageType: "OrderRequest",
        Price: parseInt(order.get("price")),
        TraderId: user.uid,
        OrderType: side,
        Amount: parseInt(order.get("amount")),
        Password: Array.from(user.pwd),
        Symbol: order.get("symbol"),
    }

    ws.send(JSON.stringify(message))
}

function OrderForm({ws, user, all_tickers}) {
    let side;
    return (
    <div style={{display: "flex", flexDirection:"column",alignItems:"center", justifyContent:"center", height:"100%", width:"100%"}}>
    <form style={{display:"flex",flexDirection:"column",alignItems:"center", justifyContent:"space-around", height:"80%", width:"100%"}}
    onSubmit={(e) => placeOrder(e, side, user, ws)}
    onKeyDown={(e) => {if (e.key == 'Enter') e.preventDefault();}}>
        
        <div style={{display:"flex",flexDirection:"column", width:"70%"}}>
        <label htmlFor="symbol">Symbol</label>
        <select id="symbol" name="symbol" style={{width:"100%", boxSizing:"border-box"}}>
            {all_tickers.map((t) => (
                <option value={t}>{t}</option>
            ))}
        </select>
        </div>
        
        <div style={{display:"flex",flexDirection:"column", width:"70%"}}>
        <label htmlFor="price">Price</label>
        <input id="price" name="price" type="number" style={{width:"100%", boxSizing:"border-box"}} required/>
        </div>

        <div style={{display:"flex",flexDirection:"column", width:"70%"}}>
        <label htmlFor="amount">Amount</label>
        <input id="amount" name="amount" type="number" style={{width:"100%", boxSizing:"border-box"}} required/>
        </div>

        <div style={{display:"flex", flexDirection:"row", width:"70%", justifyContent:"space-between"}}>
        <input style={{flex:"1", marginRight:"2px", background:"none", border:"1px solid white", color: "white",
            className:"ibm-plex-sans-bold"}}
        type="submit" value="Buy" onClick={() => side = "Buy"}/>
        <input style={{flex:"1", marginLeft:"2px", background:"none", border:"1px solid white", color: "white",
            className:"ibm-plex-sans-bold"}}
        type="submit" value="Sell" onClick={() => side = "Sell"}/>
        </div>

    </form>
    </div>
    )
}

export default OrderForm