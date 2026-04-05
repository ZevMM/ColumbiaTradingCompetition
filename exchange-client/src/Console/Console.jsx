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
  const all_tickers = Object.keys(game);
  const maxprice = game[all_tickers[0]].buy_side_limit_levels.length - 1;

  let cumsum_buy = []
  let lowbuy = game[cur_ticker].buy_side_limit_levels.findIndex((e) => e.total_volume > 0)
  let highbuy = game[cur_ticker].buy_side_limit_levels.findLastIndex((e) => e.total_volume > 0)
  game[cur_ticker].buy_side_limit_levels.slice(lowbuy, highbuy+1).reduceRight((s,c) => {
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
    <div className="console-grid">
        <div className="area-stats panel">
            <StatsBar account={account} game={game} />
        </div>
        <div className="area-order panel">
            <div className="panel-header">Order Entry</div>
            <OrderForm ws={ws} user={user} all_tickers={all_tickers} maxprice={maxprice}/>
        </div>
        <div className="area-tickers panel">
            <div className="panel-header">Markets</div>
            <Tickers cur_ticker={cur_ticker} setCur_ticker={setCur_ticker} all_tickers={all_tickers} game={game}/>
        </div>
        <div className="area-port panel">
            <div className="panel-header">Active Orders</div>
            <Portfolio ws={ws} account={account} user={user}/>
        </div>

        <div className="area-chart panel" style={{padding: 0}}>
            <PriceChart game={game} cur_ticker={cur_ticker} />
        </div>
        <div className="area-chart2 panel" style={{padding: 0}}>
            <DepthChart buyside={cumsum_buy} sellside={cumsum_sell} lowsell={lowsell} lowbuy={lowbuy}/>
        </div>
        <div className="area-book panel">
            <div className="panel-header">Order Book</div>
            <OrderBook buyside={cumsum_buy} sellside={cumsum_sell} lowsell={lowsell} lowbuy={lowbuy}/>
        </div>
    </div>
  )
}

export default Console
