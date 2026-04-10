import token from "../assets/Token.png"

const pct_change = (price_history) => {
    if(price_history.length < 1) return null;
    let s = price_history.at(0)[1];
    let e = price_history.at(-1)[1];
    let c = 100 * (e - s) / s;
    if (c > 0) return (
        <div className="ticker-change up">+{c.toFixed(2)}%</div>
    );
    if (c < 0) return (
        <div className="ticker-change down">{c.toFixed(2)}%</div>
    );
    return (<div className="ticker-change flat">{c.toFixed(2)}%</div>);
}

const bestPrices = (game, symbol) => {
    const bids = Object.keys(game[symbol].buy_side).map(Number);
    const asks = Object.keys(game[symbol].sell_side).map(Number);
    const bid = bids.length ? Math.max(...bids) : null;
    const ask = asks.length ? Math.min(...asks) : null;
    return {bid, ask};
}

function Tickers({cur_ticker, setCur_ticker, all_tickers, game, account}) {
    return (
        <div className="tickers-list">
            {
                all_tickers.map((symbol) => {
                    const {bid, ask} = bestPrices(game, symbol);
                    const pos = account?.asset_balances?.[symbol] ?? 0;
                    const posClass = pos > 0 ? 'pnl-pos' : pos < 0 ? 'pnl-neg' : '';
                    return (
                        <div key={symbol}
                        onClick={()=>setCur_ticker(symbol)}
                        className={`ticker-row${cur_ticker === symbol ? ' active' : ''}`}>
                            <div className="ticker-top">
                                <div className="ticker-symbol">{symbol}</div>
                                <div className="ticker-price">
                                    {game[symbol].price_history?.at(-1)?.[1] ?? '–'}<img src={token}/>
                                </div>
                            </div>
                            <div className="ticker-mid">
                                <div className="ticker-bidask">
                                    <span className="ba-bid">{bid ?? '–'}</span>
                                    <span className="ba-sep">/</span>
                                    <span className="ba-ask">{ask ?? '–'}</span>
                                </div>
                                {pct_change(game[symbol].price_history)}
                            </div>
                            <div className="ticker-bot">
                                <span className="ticker-pos-label">POS</span>
                                <span className={`ticker-pos ${posClass}`}>{pos}</span>
                            </div>
                        </div>
                    )
                })
            }
        </div>
    )
}

export default Tickers
