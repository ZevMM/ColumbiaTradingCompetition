import token from "../assets/Token.png"

function OrderBook({buyside, sellside, buyprices, sellprices}) {
    // Data conventions (from Console.jsx):
    //   buyprices[0] = best bid (highest), buyprices[end] = lowest
    //   sellprices[0] = best ask (lowest), sellprices[end] = highest
    const bestBid = buyprices.length > 0 ? buyprices[0] : null;
    const bestAsk = sellprices.length > 0 ? sellprices[0] : null;
    const spread = (bestBid != null && bestAsk != null) ? bestAsk - bestBid : null;
    const mid = (bestBid != null && bestAsk != null) ? ((bestBid + bestAsk) / 2) : null;

    // Render asks top-to-bottom in DESCENDING price order (best ask at the bottom, just above spread)
    const askMax = sellside.length > 0 ? sellside.at(-1) : 1;
    const askIndices = [...sellprices.keys()].reverse();

    // Render bids top-to-bottom in DESCENDING price order (best bid at the top, just below spread)
    const bidMax = buyside.length > 0 ? buyside[0] : 1;

    return (
        <div className="order-book">
            <div className="book-header">
                <div>Price <img src={token} style={{width:"10px"}}/></div>
                <div style={{textAlign: 'right'}}>Volume</div>
            </div>

            <div className="book-sell-side">
                {askIndices.map((i) => (
                    <div key={i} className={`book-row sell-row${i === 0 ? ' best-ask' : ''}`}
                        style={{background: `linear-gradient(to left, rgba(239, 83, 80, 0.25) ${sellside[i] * 100 / askMax}%, transparent ${sellside[i] * 100 / askMax}%)`}}>
                        <div>{sellprices[i]}</div><div style={{textAlign: 'right'}}>{sellside[i]}</div>
                    </div>
                ))}
            </div>

            <div className="book-spread">
                {spread != null ? (
                    <>
                        <span className="spread-mid">{mid.toFixed(1)}</span>
                        <span className="spread-label">spread {spread}</span>
                    </>
                ) : (
                    <span className="spread-label">no market</span>
                )}
            </div>

            <div className="book-buy-side">
                {buyside.map((l, i) => (
                    <div key={i} className={`book-row buy-row${i === 0 ? ' best-bid' : ''}`}
                        style={{background: `linear-gradient(to left, rgba(38, 166, 154, 0.25) ${l * 100 / bidMax}%, transparent ${l * 100 / bidMax}%)`}}>
                       <div>{buyprices[i]}</div><div style={{textAlign: 'right'}}>{l}</div>
                    </div>
                ))}
            </div>
        </div>
    )
}

export default OrderBook
