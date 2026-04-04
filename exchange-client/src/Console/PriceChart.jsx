import React from 'react';
import { useState, useEffect, useRef } from 'react';
import createPlotlyComponent from 'react-plotly.js/factory';
const Plot = createPlotlyComponent(Plotly);


function makeCandles(prices, idx, candles) {
  //no new prices
  if (idx >= prices.length) return idx;

  let open, hi, lo, close, volume;

  let end_time = Math.floor(prices.at(-1)[0] / 60);
  let t = Math.floor(prices[idx][0] / 60)

  while (t < end_time) {
      //create tick for time period t
      open = hi = lo = prices[idx][1]
      volume = prices[idx][2]
      while (Math.floor(prices[idx + 1][0] / 60) < t + 1) {
          hi = Math.max(hi, prices[idx][1])
          lo = Math.min(lo, prices[idx][1])
          volume += prices[idx][2]
          idx++
      }
      close = prices[idx][1]
      candles.push({ open, hi, lo, close, volume })
      idx++

      //fill in empty ticks until we get to next period with order
      let next = (prices[idx][0] / 60)
      while (t + 2 <= next) {
          candles.push({open: close, hi: close, lo: close, close: close, volume: 0})
          t += 1
      }

      //get start time of next period
      t = Math.floor(prices[idx][0] / 60)
  } return idx
}


const PriceChart = ({game, cur_ticker}) => {
  const priceref = useRef([])
  const candles = useRef([])
  const c_idx = useRef(0)
  const [revision, setRevision] = useState(0)

  priceref.current = game[cur_ticker].price_history


  useEffect(() => {

    candles.current = []
    setRevision(Object.keys(game).findIndex((ct) => ct == cur_ticker) * -1)
    c_idx.current = makeCandles(priceref.current, 0, candles.current)

    const intervalId = setInterval(() => {
      c_idx.current = makeCandles(priceref.current, c_idx.current, candles.current)
      setRevision(candles.current.length)
      console.log(candles.current.length)
    }, 60000);

    return () => clearInterval(intervalId);
  }, [cur_ticker]);

  const intervals = candles.current.map((_, i) => i + 1);
  const open = candles.current.map(item => item.open);
  const high = candles.current.map(item => item.hi);
  const low = candles.current.map(item => item.lo);
  const close = candles.current.map(item => item.close);
  const volume = candles.current.map(item => item.volume);

  const layout = {
    xaxis: {
      rangeslider: { visible: true, bgcolor: '#131722' },
      range: intervals.length > 22 ? [intervals[intervals.length - 22], intervals[intervals.length]] : [0, 22],
      gridcolor: '#1e2235',
      linecolor: '#2a2e3e',
      tickfont: { color: '#787b86', family: 'IBM Plex Mono', size: 10 },
    },
    yaxis: {
      title: { text: 'Price', font: { color: '#787b86', family: 'IBM Plex Mono', size: 11 } },
      gridcolor: '#1e2235',
      linecolor: '#2a2e3e',
      tickfont: { color: '#787b86', family: 'IBM Plex Mono', size: 10 },
    },
    yaxis2: {
      title: { text: 'Volume', font: { color: '#787b86', family: 'IBM Plex Mono', size: 11 } },
      overlaying: 'y',
      side: 'right',
      showgrid: false,
      linecolor: '#2a2e3e',
      tickfont: { color: '#787b86', family: 'IBM Plex Mono', size: 10 },
    },
    showlegend: false,
    autosize: true,
    margin: {
      t: 10,
      b: 10,
      l: 50,
      r: 50,
    },
    paper_bgcolor: '#131722',
    plot_bgcolor: '#131722',
  };

  const traces = [
    {
      x: intervals,
      open: open,
      high: high,
      low: low,
      close: close,
      type: 'candlestick',
      name: 'Candlestick',
      increasing: {
        line: { color: '#26a69a' },
        fillcolor: 'rgba(38, 166, 154, 0.6)',
      },
      decreasing: {
        line: { color: '#ef5350' },
        fillcolor: 'rgba(239, 83, 80, 0.6)',
      },
      yaxis: 'y2',
    },
    {
      x: intervals,
      y: volume,
      type: 'bar',
      name: 'Volume',
      yaxis: 'y',
      marker: {
        color: 'rgba(41, 98, 255, 0.25)',
      },
    },
  ];

  return (
    <Plot
      style={{width:"100%", height:"100%"}}
      data={traces}
      layout={layout}
      config={{
        displayModeBar: 'hover',
        scrollZoom: true,
        responsive: true,
      }}
      revision={revision}
    />
  );
};

export default PriceChart;
