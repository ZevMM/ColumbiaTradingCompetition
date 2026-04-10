import { useState } from "react"
import token from "../assets/Token.png"

function Portfolio({ws, account, user}) {
    const cancelAll = () => {
        for (const o of account.active_orders) {
            ws.send(JSON.stringify({
                MessageType: "CancelRequest",
                OrderId: parseInt(o.order_id),
                TraderId: user.uid,
                Price: parseInt(o.price),
                Symbol: o.symbol,
                Side: o.order_type,
                Password: Array.from(user.pwd),
            }))
        }
    }

    if (account.active_orders.length === 0) {
        return (
            <div className="portfolio portfolio-empty">
                No active orders
            </div>
        )
    }
    return (
        <div className="portfolio">
        <div className="portfolio-actions">
            <button className="btn-cancel-all" onClick={cancelAll}>Cancel All ({account.active_orders.length})</button>
        </div>
        <table>
            <thead>
            <tr>
                <th>Symbol</th>
                <th>Price <img src={token}/></th>
                <th>Volume</th>
                <th>Side</th>
            </tr>
            </thead>
            <tbody>
            {account.active_orders.map((o) => {
                return (
                    <tr key={o.order_id}>
                        <td>{o.symbol}</td>
                        <td>{o.price}</td>
                        <td>{o.amount}</td>
                        <td className={o.order_type === "Buy" ? "pnl-pos" : "pnl-neg"}>{o.order_type}</td>
                        <div className="overlay">
                            <input type="button" value="Cancel" onClick={() => {
                            ws.send(
                            JSON.stringify(
                                {
                                    MessageType: "CancelRequest",
                                    OrderId: parseInt(o.order_id),
                                    TraderId: user.uid,
                                    Price: parseInt(o.price),
                                    Symbol: o.symbol,
                                    Side: o.order_type,
                                    Password: Array.from(user.pwd),
                                }
                            ))}}/>
                        </div>
                    </tr>
                )
            })}
            </tbody>
        </table>
        </div>
    )
}

export default Portfolio
