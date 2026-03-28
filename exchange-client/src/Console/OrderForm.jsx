import token from "../assets/Token.png"

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

function OrderForm({ws, user, all_tickers, maxprice}) {
    let side;
    return (
    <form className="order-form"
    onSubmit={(e) => placeOrder(e, side, user, ws)}
    onKeyDown={(e) => {if (e.key == 'Enter') e.preventDefault();}}>

        <div className="field-group">
        <label htmlFor="symbol">Symbol</label>
        <select id="symbol" name="symbol">
            {all_tickers.map((t) => (
                <option key={t} value={t}>{t}</option>
            ))}
        </select>
        </div>

        <div className="field-group">
        <label htmlFor="price">Price <img src={token}/></label>
        <input id="price" name="price" type="number" min="0" max={maxprice} required/>
        </div>

        <div className="field-group">
        <label htmlFor="amount">Amount</label>
        <input id="amount" name="amount" type="number" min="1" required/>
        </div>

        <div className="btn-row">
        <button className="btn-buy" type="submit" onClick={() => side = "Buy"}>Buy</button>
        <button className="btn-sell" type="submit" onClick={() => side = "Sell"}>Sell</button>
        </div>

    </form>
    )
}

export default OrderForm
