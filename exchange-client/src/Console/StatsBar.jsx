import token from "../assets/Token.png"

function StatsBar({account, game}) {
    let urpl = Object.entries(account.asset_balances).reduce(
        (s, [k,v], i) => {
            return (s + (100 * v * (game[k].price_history?.at(-1)?.[1] ?? 0)) / (100 + v))
        }, 0
    )
    const netValue = urpl + account.cents_balance
    const pnlClass = urpl > 0 ? 'pnl-pos' : urpl < 0 ? 'pnl-neg' : 'pnl-flat'

    return (
        <div className="stats-bar">
            <div className="stats-section stats-pnl">
                <div>
                    <div className="stat-label">Net Account Value</div>
                    <div className="stat-value stat-big"><img src={token}/>{netValue.toFixed(0)}</div>
                </div>
                <div>
                    <div className="stat-label">Unrealized P/L</div>
                    <div className={`stat-value stat-big ${pnlClass}`}><img src={token}/>{urpl >= 0 ? '+' : ''}{urpl.toFixed(0)}</div>
                </div>
                <div>
                    <div className="stat-label">Available Margin</div>
                    <div className="stat-value stat-big"><img src={token}/>{account.net_cents_balance}</div>
                </div>
            </div>
            <div className="stats-section">
                <div>
                    <div className="stat-label">Cash</div>
                    <div className="stat-value"><img src={token}/>{account.cents_balance}</div>
                </div>
                {Object.entries(account.asset_balances).map(([k,v]) => (
                    <div key={k}>
                        <div className="stat-label">{k}</div>
                        <div className={`stat-value ${v > 0 ? 'pnl-pos' : v < 0 ? 'pnl-neg' : ''}`}>{v}</div>
                    </div>
                ))}
            </div>
        </div>
    )
}

export default StatsBar
