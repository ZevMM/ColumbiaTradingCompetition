import token from "../assets/Token.png"

function OrderBook({buyside, sellside, buyprices, sellprices}) {
    return (
        <div className="order-book">
            <div id="buyside" style={{flex: 1}}>
                {buyside
                    .map((l, i) => (
                        <div key={i} className="book-row buy-row"
                            style={{background: `linear-gradient(to right, rgba(38, 166, 154, 0.25) ${l * 100 / buyside[0]}%, transparent ${l * 100 / buyside[0]}%)`}}>
                           <div>{buyprices[i]}</div><div>{l}</div>
                        </div>
                    ))}
            </div>

            <div className="book-header">
                <div>Price <img src={token} style={{width:"10px"}}/></div>
                <div>Volume</div>
            </div>

            <div className="book-sell-side" style={{flex: 1}}>
                {sellside
                    .map((l, i) => (
                    <div key={i} className="book-row sell-row"
                        style={{background: `linear-gradient(to right, rgba(239, 83, 80, 0.25) ${l * 100 / sellside.at(-1)}%, transparent ${l * 100 / sellside.at(-1)}%)`}}>
                        <div>{sellprices[i]}</div><div>{l}</div>
                    </div>
                    ))}
            </div>
        </div>
    )
}

export default OrderBook
