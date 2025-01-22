function OrderBook({buyside, sellside, lowsell, lowbuy}) {
    return (
        <>
            <div style={{height:"50%", display:"flex", flexDirection:"column-reverse", overflowY:"auto"}}>
            <div>
                {buyside
                    .map((l, i) => {return (
                        l > 0 ? 
                        <div style={{background: `linear-gradient(to right, rgba(0, 255, 0, 0.3) ${l * 100 / buyside[0]}%, rgba(0, 0, 0, 0) ${1 - (l * 100 / buyside[0])}%)`}}>{`${i + Math.max(lowbuy -1 , 0)} ${l}`}</div>
                        : null
                        )})}
            </div>
            </div>
            <div className="ibm-plex-sans-bold">Price Volume</div>
            <div style={{height:"50%", overflowY:"auto"}}>
                {sellside
                    .map((l, i) => {return (
                    l > 0 ? 
                    <div style={{background: `linear-gradient(to right, rgba(255, 0, 0, 0.3) ${l * 100 / sellside.at(-1)}%, rgba(0, 0, 0, 0) ${1 - (l * 100 / sellside.at(-1))}%)`}}>{`${i + lowsell} ${l}`}</div>
                    : null
                    )})}
            </div>

        </>
    )
}

export default OrderBook