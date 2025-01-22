import { useState } from "react"

function Portfolio({ws, account, user}) {
    //const [tab, setTab] = useState(1)
    //return tab == 1 ? 
    return (
        <div style={{width:"100%", height:"98%", overflowY:"auto"}}>
        <div>Active Orders</div>
        <table id="portfolio" style={{width: "100%"}}>
            <thead>
            <tr style={{position: "sticky", top: 0, zIndex: 1, background:"rgb(0, 0, 225)"}}>
                <th>Symbol</th>
                <th>Price</th>
                <th>Volume</th>
                <th>Side</th>
            </tr>
            </thead>
            <tbody>
            {account.active_orders.map((o) => {
                return (
                    <tr>
                        <td>{o.symbol}</td>
                        <td>{o.price}</td>
                        <td>{o.amount}</td>
                        <td>{o.order_type}</td>
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
    /*
    :
    (
        <>
        <span onClick={()=>setTab(1)}>Orders</span><span>Positions</span>
        <div>
            Positions
        </div>
        </>
    )
    */
}

export default Portfolio