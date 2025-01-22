function StatsBar({account, game}) {
    let urpl = Object.entries(account.asset_balances).reduce(
        (s, [k,v], i) => {
            return (s + v * (game[k].price_history?.at(-1)?.[1] ?? 0))
        }, 0
    )

    return (
        <div style={{display:"flex", width:"100%", flexDirection:"row", justifyContent:"space-around", color:"white"}}>
            <div style={{flex:2, marginLeft:"10px", marginRight:"50px"}}>
                <div style={{}}>Assets</div>
                <div style={{display:"flex", flexDirection:"row", justifyContent:"space-between", flex:1}}>
                    <div>
                        <div className="ibm-plex-sans-bold" style={{}}>Cash</div>
                        <div style={{fontSize:"26px", fontWeight:"bold"}}>${account.cents_balance}</div>
                    </div>
                    {Object.entries(account.asset_balances).map(([k,v]) => {
                        return (
                            <div>
                            <div className="ibm-plex-sans-bold" style={{fontSize:"20px"}}>{k}</div>
                            <div style={{fontSize:"26px", fontWeight:"bold"}}>{v}</div>
                            </div>
                        )
                    })}
                </div>
            </div>
            <div style={{flex:3, marginLeft:"50px", marginRight:"10px"}}>
                <div style={{}}>Account Statistics</div>
                <div style={{display:"flex", flexDirection:"row", justifyContent:"space-between", flex:1}}>
                    <div>
                        <div className="ibm-plex-sans-bold" style={{fontSize:"20px"}}>Unrealized P/L</div>
                        <div style={{fontSize:"26px", fontWeight:"bold"}}>${urpl}</div>
                    </div>
                    <div>
                        <div className="ibm-plex-sans-bold" style={{fontSize:"20px"}}>Net Account Value</div>
                        <div style={{fontSize:"26px", fontWeight:"bold"}}>${urpl + account.cents_balance}</div>
                    </div>
                    <div>
                        <div className="ibm-plex-sans-bold" style={{fontSize:"20px"}}>Available Margin</div>
                        <div style={{fontSize:"26px", fontWeight:"bold"}}>${account.net_cents_balance}</div>
                    </div>
                </div>
            </div>
        </div>
    )
}

export default StatsBar