import { useState, useEffect } from 'react'
import Tickers from './Tickers';
import OrderBook from './OrderBook';
import PriceChart from './PriceChart';
import DepthChart from './DepthChart';
import OrderForm from './OrderForm';
import Portfolio from './Portfolio';
import StatsBar from './StatsBar';

function Console({ws, user, game, account}) {
  if (!game || !account) { return <div>Loading...</div> }
  
  const [cur_ticker, setCur_ticker] = useState(Object.keys(game)[0]);
  const all_tickers = Object.keys(game)

  let cumsum_buy = []
  let lowbuy = game[cur_ticker].buy_side_limit_levels.findIndex((e) => e.total_volume > 0)
  let highbuy = game[cur_ticker].buy_side_limit_levels.findLastIndex((e) => e.total_volume > 0)
  game[cur_ticker].buy_side_limit_levels.slice(Math.max(lowbuy - 1, 0), highbuy + 1).reduceRight((s,c) => {
    let cs = s + c.total_volume
    cumsum_buy.unshift(cs)
    return cs
  }, 0)
  
  let cumsum_sell = []
  let lowsell = game[cur_ticker].sell_side_limit_levels.findIndex((e) => e.total_volume > 0)
  let highsell = game[cur_ticker].sell_side_limit_levels.findLastIndex((e) => e.total_volume > 0)
  game[cur_ticker].sell_side_limit_levels.slice(lowsell, highsell+2).reduce((s,c) => {
    let cs = s + c.total_volume
    cumsum_sell.push(cs)
    return cs
  }, 0)
  
  
  return (
    <div style={{fontFamily:"IBM Plex Sans Condensed", color:"white"}}>
        <div style={{background: "#19191fff", width:"100%", height:"10%", "left": 0, "top":0, position:"absolute", boxShadow:"0 0px 2px rgba(0, 0, 0, 1)"}}>
            <StatsBar account={account} game={game} />
        </div>
        <div style={{background: "rgb(10, 10, 18)", width:"15%", height:"38%", "left": 0, "top":"10%", position:"absolute"}}>
            <OrderForm ws={ws} user={user} all_tickers={all_tickers}/>
        </div>
        <div style={{background: "rgb(10, 10, 18)", width:"15%", height:"38%", "left": "15%", "top":"10%", position:"absolute", borderLeft:"1px solid white"}}>
            <Tickers cur_ticker={cur_ticker} setCur_ticker={setCur_ticker} all_tickers={all_tickers} game={game}/>
        </div>
        <div style={{background: "rgb(10, 10, 18)", width:"30%", height:"52%", "left": 0, "top":"48%", position:"absolute", borderTop:"1px solid white"}}>
            <Portfolio ws={ws} account={account} user={user}/>
        </div>

        <div style={{background: "rgb(10, 10, 18)", width:"57%", height:"54%", "left": "30.5%", "top":"11%", position:"absolute"}}>
            <PriceChart game={game} cur_ticker={cur_ticker} />
        </div>
        <div style={{background: "rgb(10, 10, 18)", width:"57%", height:"34%", "left": "30.5%", "top":"65%", position:"absolute"}}>
            <DepthChart buyside={cumsum_buy} sellside={cumsum_sell} lowsell={lowsell} lowbuy={lowbuy}/>
        </div>
        <div style={{background: "rgb(10, 10, 18)", width:"12%", height:"88%", "left": "87.5%", "top":"11%", position:"absolute"}}>
            <OrderBook buyside={cumsum_buy} sellside={cumsum_sell} lowsell={lowsell} lowbuy={lowbuy}/>
        </div>

        <div id="glow" style={{position:"absolute", left:"30.5%", top:"11%", bottom:"1%", right:"1%"}}>
        </div>
    </div>
  )
}

export default Console
