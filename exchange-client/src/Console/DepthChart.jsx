import React from 'react';
import createPlotlyComponent from 'react-plotly.js/factory';
import { useState } from 'react';

const Plot = createPlotlyComponent(Plotly);
const DepthChart = ({buyside, sellside, lowsell, lowbuy}) => {
  
  const buyprices = buyside.map((_, i) => i + Math.max(lowbuy - 1, 0));
  const sellprices = sellside.map((_, i) => i + lowsell); 

  const layout = {
    xaxis: {
      title: 'Price',
      showgrid: true,
    },
    yaxis: {
      title: 'Volume',
      autorange: true
    },
    showlegend: false,
    autosize: true,
    margin: {
      t: 10, 
      b: 30, 
      l: 50, 
      r: 10, 
    },
    paper_bgcolor: 'rgba(0,0,0,0)',
    plot_bgcolor: 'rgba(0,0,0,0)',
  };

  const traces = [
    {
      x: buyprices,
      y: buyside,
      type: 'scatter',
      mode: 'lines', 
      name: 'Line 1 (Green)',
      line: { color: 'green', shape: 'hv'}, 
      fill: 'tozeroy', 
      fillcolor: 'rgba(0, 255, 0, 0.3)', 
    },
    {
      x: sellprices,
      y: sellside,
      type: 'scatter',
      mode: 'lines', 
      name: 'Line 2 (Red)',
      line: { color: 'red', shape: 'hv'}, 
      fill: 'tozeroy', 
      fillcolor: 'rgba(255, 0, 0, 0.3)', 
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
