import token from "../assets/Token.png"

const pct_change = (price_history) => {
    if(price_history.length < 1) return;
    let s = price_history.at(0)[1];
    let e = price_history.at(-1)[1];
    let c = 100 * (e - s) / s;
    if (c > 0) return (
    <div style={{color: 'transparent', textShadow: "0 0 0 green", fontSize:"18px", marginLeft:"2px"}}>
    +{c.toFixed(2)}%🔺
    </div>
    );
    if (c < 0) return (
    <div style={{color: 'transparent', textShadow: "0 0 0 red", fontSize:"18px", marginLeft:"2px"}}>
    {c.toFixed(2)}%🔻
    </div>);
    return (<div style={{color:"grey", fontSize:"18px", marginLeft:"2px"}}>{c.toFixed(2)}%</div>);
}

function Tickers({cur_ticker, setCur_ticker, all_tickers, game}) {
    return (
        <div style={{display: "flex",
                    flexDirection:"column",
                    justifyContent:"space-around",
                    width:"100%",
                    height:"100%",
                    fontSize:"36px"
                    }}>
            {
                all_tickers.map((symbol) => {return (
                    <div key={symbol}
                    onClick={()=>setCur_ticker(symbol)}
                    style= {cur_ticker == symbol ? {background: "#19191fff", width:"100%", borderRight:"20px solid rgb(10, 10, 18)", display:"flex", flexDirection:"row", justifyContent:"space-around"} : 
                    { width:"100%", display:"flex", flexDirection:"row", justifyContent:"space-around"}}>
                    <div className="ibm-plex-sans-bold">{symbol}</div>
                    <div>
                        <div>{game[symbol].price_history?.at(-1)?.[1]}<img src={token} style={{width:"20px"}}/></div>
                        {pct_change(game[symbol].price_history)}
                    </div>
                    </div>
                )})
            }
        </div>
    )

}

export default Tickers