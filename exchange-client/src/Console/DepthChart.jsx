import React from 'react';
import createPlotlyComponent from 'react-plotly.js/factory';
import { useState } from 'react';

const Plot = createPlotlyComponent(Plotly);
const DISPLAY_MARGIN = 30;

const DepthChart = ({buyside, sellside, buyprices, sellprices}) => {
  const bestBid = buyprices.length > 0 ? buyprices[buyprices.length - 1] : null;
  const bestAsk = sellprices.length > 0 ? sellprices[0] : null;

  let xrange;
  if (bestBid !== null && bestAsk !== null) {
    xrange = [bestBid - DISPLAY_MARGIN, bestAsk + DISPLAY_MARGIN];
  } else if (bestBid !== null) {
    xrange = [bestBid - DISPLAY_MARGIN, bestBid + DISPLAY_MARGIN];
  } else if (bestAsk !== null) {
    xrange = [bestAsk - DISPLAY_MARGIN, bestAsk + DISPLAY_MARGIN];
  }

  const layout = {
    xaxis: {
      title: { text: 'Price', font: { color: '#787b86', family: 'IBM Plex Mono', size: 11 } },
      showgrid: true,
      gridcolor: '#1e2235',
      linecolor: '#2a2e3e',
      tickfont: { color: '#787b86', family: 'IBM Plex Mono', size: 10 },
      ...(xrange && { range: xrange }),
    },
    yaxis: {
      title: { text: 'Volume', font: { color: '#787b86', family: 'IBM Plex Mono', size: 11 } },
      autorange: true,
      gridcolor: '#1e2235',
      linecolor: '#2a2e3e',
      tickfont: { color: '#787b86', family: 'IBM Plex Mono', size: 10 },
    },
    showlegend: false,
    autosize: true,
    margin: {
      t: 10,
      b: 30,
      l: 50,
      r: 10,
    },
    paper_bgcolor: '#131722',
    plot_bgcolor: '#131722',
  };

  const traces = [
    {
      x: buyprices,
      y: buyside,
      type: 'scatter',
      mode: 'lines',
      name: 'Bids',
      line: { color: '#26a69a', shape: 'hv'},
      fill: 'tozeroy',
      fillcolor: 'rgba(38, 166, 154, 0.15)',
    },
    {
      x: sellprices,
      y: sellside,
      type: 'scatter',
      mode: 'lines',
      name: 'Asks',
      line: { color: '#ef5350', shape: 'hv'},
      fill: 'tozeroy',
      fillcolor: 'rgba(239, 83, 80, 0.15)',
    },
  ];

  return (

      <Plot
        style={{ width: '100%', height: '100%' }}
        data={traces}
        layout={layout}
        config={{
          displayModeBar: false,
          scrollZoom: false,
          responsive: true,
        }}
      />
  );
};

export default DepthChart;
