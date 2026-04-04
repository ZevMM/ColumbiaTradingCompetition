import { useState, useEffect, useRef } from 'react'
import Tickers from './Tickers';
import OrderBook from './OrderBook';
import PriceChart from './PriceChart';
import DepthChart from './DepthChart';
import OrderForm from './OrderForm';
import Portfolio from './Portfolio';
import StatsBar from './StatsBar';

const DEPTH_CHART_FPS = 10;

function Console({ws, user, game, account}) {
  if (!game || !account) { return <div>Loading...</div> }

  const [cur_ticker, setCur_ticker] = useState(Object.keys(game)[0]);
  const all_tickers = Object.keys(game);

  // Throttled game snapshot used only by the Plotly depth chart.
  // Text-based components (OrderBook, StatsBar) use `game` directly at full speed.
  const [chartGame, setChartGame] = useState(game);
  const pendingGame = useRef(game);
  useEffect(() => {
    pendingGame.current = game;
  }, [game]);
  useEffect(() => {
    const id = setInterval(() => {
      setChartGame(pendingGame.current);
    }, 1000 / DEPTH_CHART_FPS);
    return () => clearInterval(id);
  }, []);

  // Order book text display — uses live `game` at full speed (cheap renders)
  const buySide = game[cur_ticker].buy_side;
  const sellSide = game[cur_ticker].sell_side;

  const buyPrices = Object.keys(buySide).map(Number).sort((a, b) => a - b);
  const lowbuy = buyPrices.length > 0 ? buyPrices[0] : 0;
  const highbuy = buyPrices.length > 0 ? buyPrices[buyPrices.length - 1] : 0;

  let cumsum_buy = [];
  if (buyPrices.length > 0) {
    let runsum = 0;
    for (let p = highbuy; p >= Math.max(lowbuy - 1, 0); p--) {
      runsum += buySide[p] || 0;
      cumsum_buy.unshift(runsum);
    }
  }

  const sellPrices = Object.keys(sellSide).map(Number).sort((a, b) => a - b);
  const lowsell = sellPrices.length > 0 ? sellPrices[0] : 0;
  const highsell = sellPrices.length > 0 ? sellPrices[sellPrices.length - 1] : 0;

  let cumsum_sell = [];
  if (sellPrices.length > 0) {
    let runsum = 0;
    for (let p = lowsell; p <= highsell + 1; p++) {
      runsum += sellSide[p] || 0;
      cumsum_sell.push(runsum);
    }
  }

  // Depth chart — uses throttled `chartGame` to cap Plotly re-renders at DEPTH_CHART_FPS
  const chartBuySide = chartGame[cur_ticker].buy_side;
  const chartSellSide = chartGame[cur_ticker].sell_side;

  const chartBuyPrices = Object.keys(chartBuySide).map(Number).sort((a, b) => a - b);
  const chartLowbuy = chartBuyPrices.length > 0 ? chartBuyPrices[0] : 0;
  const chartHighbuy = chartBuyPrices.length > 0 ? chartBuyPrices[chartBuyPrices.length - 1] : 0;

  let chart_cumsum_buy = [];
  if (chartBuyPrices.length > 0) {
    let runsum = 0;
    for (let p = chartHighbuy; p >= Math.max(chartLowbuy - 1, 0); p--) {
      runsum += chartBuySide[p] || 0;
      chart_cumsum_buy.unshift(runsum);
    }
  }

  const chartSellPrices = Object.keys(chartSellSide).map(Number).sort((a, b) => a - b);
  const chartLowsell = chartSellPrices.length > 0 ? chartSellPrices[0] : 0;
  const chartHighsell = chartSellPrices.length > 0 ? chartSellPrices[chartSellPrices.length - 1] : 0;

  let chart_cumsum_sell = [];
  if (chartSellPrices.length > 0) {
    let runsum = 0;
    for (let p = chartLowsell; p <= chartHighsell + 1; p++) {
      runsum += chartSellSide[p] || 0;
      chart_cumsum_sell.push(runsum);
    }
  }



  return (
    <div className="console-grid">
        <div className="area-stats panel">
            <StatsBar account={account} game={game} />
        </div>
        <div className="area-order panel">
            <div className="panel-header">Order Entry</div>
            <OrderForm ws={ws} user={user} all_tickers={all_tickers}/>
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
            <DepthChart buyside={chart_cumsum_buy} sellside={chart_cumsum_sell} lowsell={chartLowsell} lowbuy={chartLowbuy}/>
        </div>
        <div className="area-book panel">
            <div className="panel-header">Order Book</div>
            <OrderBook buyside={cumsum_buy} sellside={cumsum_sell} lowsell={lowsell} lowbuy={lowbuy}/>
        </div>
    </div>
  )
}

export default Console
