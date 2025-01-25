import token from "../assets/Token.png"

function StatsBar({account, game}) {
    let urpl = Object.entries(account.asset_balances).reduce(
        (s, [k,v], i) => {
            return (s + v * (game[k].price_history?.at(-1)?.[1] ?? 0))
        }, 0
    )

    return (
        <div style={{display:"flex", width:"100%", flexDirection:"row", justifyContent:"space-around", color:"white"}}>
            <div style={{flex:2, marginLeft:"1vw", marginRight:"2.5vw"}}>
                <div style={{fontSize:"1.7vh"}}>Assets</div>
                <div style={{display:"flex", flexDirection:"row", justifyContent:"space-between", flex:1}}>
                    <div>
                        <div className="ibm-plex-sans-bold" style={{fontSize:"2.3vh"}}>Cash</div>
                        <div style={{fontSize:"3vh", fontWeight:"bold", display:"flex", alignItems:"center"}}><img src={token} style={{width: '2.5vh'}}/>{account.cents_balance}</div>
                    </div>
                    {Object.entries(account.asset_balances).map(([k,v]) => {
                        return (
                            <div>
                            <div className="ibm-plex-sans-bold" style={{fontSize:"2.3vh"}}>{k}</div>
                            <div style={{fontSize:"3vh", fontWeight:"bold"}}>{v}</div>
                            </div>
                        )
                    })}
                </div>
            </div>
            <div style={{flex:3, marginLeft:"2.5vw", marginRight:"1vw"}}>
                <div style={{fontSize:"1.7vh"}}>Account Statistics</div>
                <div style={{display:"flex", flexDirection:"row", justifyContent:"space-between", flex:1}}>
                    <div>
                        <div className="ibm-plex-sans-bold" style={{fontSize:"2.3vh"}}>Unrealized P/L</div>
                        <div style={{fontSize:"3vh", fontWeight:"bold", display:"flex", alignItems:"center"}}><img src={token} style={{width: '2.5vh'}}/>{urpl}</div>
                    </div>
                    <div>
                        <div className="ibm-plex-sans-bold" style={{fontSize:"2.3vh"}}>Net Account Value</div>
                        <div style={{fontSize:"3vh", fontWeight:"bold", display:"flex", alignItems:"center"}}><img src={token} style={{width: '2.5vh'}}/>{urpl + account.cents_balance}</div>
                    </div>
                    <div>
                        <div className="ibm-plex-sans-bold" style={{fontSize:"2.3vh"}}>Available Margin</div>
                        <div style={{fontSize:"3vh", fontWeight:"bold", display:"flex", alignItems:"center"}}><img src={token} style={{width: '2.5vh'}}/>{account.net_cents_balance}</div>
                    </div>
                </div>
            </div>
        </div>
    )
}

export default StatsBar