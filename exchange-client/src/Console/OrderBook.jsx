import token from "../assets/Token.png"

function OrderBook({buyside, sellside, lowsell, lowbuy}) {
    return (
        <>
            <div style={{height:"48.5%", display:"flex", flexDirection:"column-reverse", overflowY:"auto"}}>
            <div>
                {buyside
                    .map((l, i) => {return (
                        l > 0 ? 
                        <div style={{background: `linear-gradient(to right, rgba(0, 255, 0, 0.3) ${l * 100 / buyside[0]}%, rgba(0, 0, 0, 0) ${1 - (l * 100 / buyside[0])}%)`}}>
                           
                           <div style={{width:"100%", display:"flex", flexDirection:"row"}}><div style={{flex:1}}>{i + Math.max(lowbuy -1 , 0)}</div><div style={{flex:1}}>{l}</div></div>
                            
                        </div>
                        : null
                )})}
            </div>
            </div>

            <div className="ibm-plex-sans-bold" style={{height:"3%", width:"100%", display:"flex", flexDirection:"row"}}>
                
                <div style={{flex:1}}>Price (<img src={token} style={{width:"12px"}}/>)</div>
                
                <div style={{flex:1}}>Volume</div>
                
            </div>
            
            <div style={{height:"48.5%", overflowY:"auto"}}>
                {sellside
                    .map((l, i) => {return (
                    l > 0 ? 
                    <div style={{background: `linear-gradient(to right, rgba(255, 0, 0, 0.3) ${l * 100 / sellside.at(-1)}%, rgba(0, 0, 0, 0) ${1 - (l * 100 / sellside.at(-1))}%)`}}>
                        
                        <div style={{width:"100%", display:"flex", flexDirection:"row"}}><div style={{flex:1}}>{i + lowsell}</div><div style={{flex:1}}>{l}</div></div>
                        
                    </div>
                    : null
                    )})}
            </div>

        </>
    )
}

export default OrderBook