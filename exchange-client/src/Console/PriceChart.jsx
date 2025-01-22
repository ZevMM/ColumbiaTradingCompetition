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
      console.log("running interval")
      c_idx.current = makeCandles(priceref.current, c_idx.current, candles.current)
      setRevision(candles.current.length)
      console.log(candles.current.length)
    }, 60000);

    return () => clearInterval(intervalId);
  }, [cur_ticker]);

  const intervals = candles.current.map((_, i) => i);
  const open = candles.current.map(item => item.open);
  const high = candles.current.map(item => item.hi);
  const low = candles.current.map(item => item.lo);
  const close = candles.current.map(item => item.close);
  const volume = candles.current.map(item => item.volume);

  const layout = {
    xaxis: {
      rangeslider: { visible: true },
    },
    yaxis: {
      title: 'Price',
    },
    yaxis2: {
      title: 'Volume',
      overlaying: 'y',
      side: 'right',
      showgrid: false, // Disable gridlines on secondary y-axis
    },
    showlegend: false,
    autosize: true,
    margin: {
      t: 10, // top margin (reduce it further if needed)
      b: 10, // bottom margin (reduce it further if needed)
      l: 50, // left margin (reduce it further if needed)
      r: 50, // right margin (reduce it further if needed)
    },
    paper_bgcolor: 'rgba(0,0,0,0)',
    plot_bgcolor: 'rgba(0,0,0,0)'
  };

  // Define the traces for Candlestick and Bar chart
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
        line: { color: 'green' }, // Solid color for increasing candles (green)
        fillcolor: 'rgba(0, 255, 0, 0.6)', // Solid fill color for increasing candles
      },
      decreasing: {
        line: { color: 'red' }, // Solid color for decreasing candles (red)
        fillcolor: 'rgba(255, 0, 0, 0.6)', // Solid fill color for decreasing candles
      },
      yaxis: 'y2', // This ensures the candlestick uses the secondary y-axis
    },
    {
      x: intervals,
      y: volume,
      type: 'bar',
      name: 'Volume',
      yaxis: 'y',
      marker: {
        color: 'rgba(255, 123, 0, 0.3)', // Semi-transparent blue for volume bars
      }, // This places the volume chart on the primary y-axis
    },
  ];

  return (
    <Plot
      style={{width:"100%", height:"100%"}}
      data={traces}
      layout={layout}
      config={{
        displayModeBar: true,
        scrollZoom: true,
        responsive: true,
      }}
      revision={revision.current}
    />
  );
};

export default PriceChart;
