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

function Tickers({cur_ticker, setCur_ticker, all_tickers, game}) {
    return (
        <div className="tickers-list">
            {
                all_tickers.map((symbol) => (
                    <div key={symbol}
                    onClick={()=>setCur_ticker(symbol)}
                    className={`ticker-row${cur_ticker === symbol ? ' active' : ''}`}>
                        <div className="ticker-symbol">{symbol}</div>
                        <div>
                            <div className="ticker-price">
                                {game[symbol].price_history?.at(-1)?.[1]}<img src={token}/>
                            </div>
                            {pct_change(game[symbol].price_history)}
                        </div>
                    </div>
                ))
            }
        </div>
    )
}

export default Tickers
