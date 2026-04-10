import token from "../assets/Token.png"

function formatTime(ts) {
    const d = new Date(ts);
    const hh = String(d.getHours()).padStart(2, '0');
    const mm = String(d.getMinutes()).padStart(2, '0');
    const ss = String(d.getSeconds()).padStart(2, '0');
    return `${hh}:${mm}:${ss}`;
}

function TradeHistory({trades}) {
    if (!trades || trades.length === 0) {
        return (
            <div className="trade-history trade-history-empty">
                No fills yet
            </div>
        )
    }
    return (
        <div className="trade-history">
            <table>
                <thead>
                    <tr>
                        <th>Time</th>
                        <th>Symbol</th>
                        <th>Side</th>
                        <th>Qty</th>
                        <th>Price <img src={token}/></th>
                    </tr>
                </thead>
                <tbody>
                    {trades.map((t, i) => (
                        <tr key={i}>
                            <td>{formatTime(t.ts)}</td>
                            <td>{t.symbol}</td>
                            <td className={t.side === "Buy" ? "pnl-pos" : "pnl-neg"}>{t.side}</td>
                            <td>{t.amount}</td>
                            <td>{t.price}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    )
}

export default TradeHistory
