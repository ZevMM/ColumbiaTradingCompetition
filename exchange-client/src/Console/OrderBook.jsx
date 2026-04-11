import token from "../assets/Token.png"

function OrderBook({buyside, sellside, buyprices, sellprices}) {
    // Data conventions from Console.jsx:
    //   buyprices is ASCENDING (lowest first, highest=best bid last)
    //   sellprices is ASCENDING (lowest=best ask first, highest last)
    //   buyside[i] = cumulative volume at buyprices[i] AND ABOVE
    //   sellside[i] = cumulative volume at sellprices[i] AND BELOW
    const bestBid = buyprices.length > 0 ? buyprices[buyprices.length - 1] : null;
    const bestAsk = sellprices.length > 0 ? sellprices[0] : null;
    const spread = (bestBid != null && bestAsk != null) ? bestAsk - bestBid : null;
    const mid = (bestBid != null && bestAsk != null) ? ((bestBid + bestAsk) / 2) : null;

    // Render asks top-to-bottom in DESCENDING price order (highest at top, best ask at bottom).
    const askMax = sellside.length > 0 ? sellside.at(-1) : 1;
    const askIndices = [...sellprices.keys()].reverse();

    // Render bids top-to-bottom in DESCENDING price order (best bid at top, lowest at bottom).
    const bidMax = buyside.length > 0 ? buyside[0] : 1;
    const bidIndices = [...buyprices.keys()].reverse();

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
                {bidIndices.map((i) => (
                    <div key={i} className={`book-row buy-row${i === buyprices.length - 1 ? ' best-bid' : ''}`}
                        style={{background: `linear-gradient(to left, rgba(38, 166, 154, 0.25) ${buyside[i] * 100 / bidMax}%, transparent ${buyside[i] * 100 / bidMax}%)`}}>
                       <div>{buyprices[i]}</div><div style={{textAlign: 'right'}}>{buyside[i]}</div>
                    </div>
                ))}
            </div>
        </div>
    )
}

export default OrderBook
