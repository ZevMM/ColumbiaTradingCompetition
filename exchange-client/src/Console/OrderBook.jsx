import token from "../assets/Token.png"

function OrderBook({buyside, sellside, lowsell, lowbuy}) {
    return (
        <div className="order-book">
            <div id="buyside" style={{flex: 1}}>
                {buyside
                    .map((l, i) => (
                        l > 0 ?
                        <div key={i} className="book-row buy-row"
                            style={{background: `linear-gradient(to right, rgba(38, 166, 154, 0.25) ${l * 100 / buyside[0]}%, transparent ${l * 100 / buyside[0]}%)`}}>
                           <div>{i + lowbuy}</div><div>{l}</div>
                        </div>
                        : null
                    ))}
            </div>

            <div className="book-header">
                <div>Price <img src={token} style={{width:"10px"}}/></div>
                <div>Volume</div>
            </div>

            <div className="book-sell-side" style={{flex: 1}}>
                {sellside
                    .map((l, i) => (
                    l > 0 ?
                    <div key={i} className="book-row sell-row"
                        style={{background: `linear-gradient(to right, rgba(239, 83, 80, 0.25) ${l * 100 / sellside.at(-1)}%, transparent ${l * 100 / sellside.at(-1)}%)`}}>
                        <div>{i + lowsell}</div><div>{l}</div>
                    </div>
                    : null
                    ))}
            </div>
        </div>
    )
}

export default OrderBook
