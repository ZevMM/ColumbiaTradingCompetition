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
  let cumsum_buy = [];
  let buy_price_axis = [];
  if (buyPrices.length > 0) {
    let runsum = 0;
    for (let i = buyPrices.length - 1; i >= 0; i--) {
      runsum += buySide[buyPrices[i]];
      cumsum_buy.unshift(runsum);
      buy_price_axis.unshift(buyPrices[i]);
    }
  }

  const sellPrices = Object.keys(sellSide).map(Number).sort((a, b) => a - b);
  let cumsum_sell = [];
  let sell_price_axis = [];
  if (sellPrices.length > 0) {
    let runsum = 0;
    for (let i = 0; i < sellPrices.length; i++) {
      runsum += sellSide[sellPrices[i]];
      cumsum_sell.push(runsum);
      sell_price_axis.push(sellPrices[i]);
    }
  }

  // Depth chart — uses throttled `chartGame` to cap Plotly re-renders at DEPTH_CHART_FPS
  const chartBuySide = chartGame[cur_ticker].buy_side;
  const chartSellSide = chartGame[cur_ticker].sell_side;

  const chartBuyPrices = Object.keys(chartBuySide).map(Number).sort((a, b) => a - b);
  let chart_cumsum_buy = [];
  let chart_buy_price_axis = [];
  if (chartBuyPrices.length > 0) {
    let runsum = 0;
    for (let i = chartBuyPrices.length - 1; i >= 0; i--) {
      runsum += chartBuySide[chartBuyPrices[i]];
      chart_cumsum_buy.unshift(runsum);
      chart_buy_price_axis.unshift(chartBuyPrices[i]);
    }
  }

  const chartSellPrices = Object.keys(chartSellSide).map(Number).sort((a, b) => a - b);
  let chart_cumsum_sell = [];
  let chart_sell_price_axis = [];
  if (chartSellPrices.length > 0) {
    let runsum = 0;
    for (let i = 0; i < chartSellPrices.length; i++) {
      runsum += chartSellSide[chartSellPrices[i]];
      chart_cumsum_sell.push(runsum);
      chart_sell_price_axis.push(chartSellPrices[i]);
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
            <DepthChart buyside={chart_cumsum_buy} sellside={chart_cumsum_sell} buyprices={chart_buy_price_axis} sellprices={chart_sell_price_axis}/>
        </div>
        <div className="area-book panel">
            <div className="panel-header">Order Book</div>
            <OrderBook buyside={cumsum_buy} sellside={cumsum_sell} buyprices={buy_price_axis} sellprices={sell_price_axis}/>
        </div>
    </div>
  )
}

export default Console
