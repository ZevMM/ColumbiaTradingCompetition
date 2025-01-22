// k6 run k6websockets.js

import ws from 'k6/ws';
import { check } from 'k6';
import exec from 'k6/execution';

// maxes out around 10.7k/s

export let options = {
  scenarios: {
    trader1: {
      exec: 'buy_order',
      executor: 'constant-vus',
      vus: 1,
      duration: '3s',
      gracefulStop: '5s',
      env: {
        URL: 'ws://127.16.123.1:4000/orders/ws',
        TRADERID: 'Columbia_A',
        PASSWORD: 'cu_a',
      }
    },
    trader2: {
      exec: 'buy_order',
      executor: 'constant-vus',
      vus: 1,
      duration: '3s',
      gracefulStop: '5s',
      env: {
        URL: 'ws://127.16.123.2:4000/orders/ws',
        TRADERID: 'Columbia_B',
        PASSWORD: 'cu_b',
      }
    },
    trader3: {
      exec: 'buy_order',
      executor: 'constant-vus',
      vus: 1,
      duration: '3s',
      gracefulStop: '5s',
      env: {
        URL: 'ws://127.16.123.3:4000/orders/ws',
        TRADERID: 'Columbia_C',
        PASSWORD: 'cu_c',
      }
    },
    trader4: {
      exec: 'buy_order',
      executor: 'constant-vus',
      vus: 1,
      duration: '3s',
      gracefulStop: '5s',
      env: {
        URL: 'ws://127.16.123.4:4000/orders/ws',
        TRADERID: 'Columbia_D',
        PASSWORD: 'cu_d',
      }
    }
  },
};

function getRandomInt(max) {
  return Math.floor(Math.random() * max);
}

export function buy_order() {
  // url = __ENV.URL;
  // url = 'ws://127.16.123.1:4000/orders/ws';
  let l_res = 0;
  const res = ws.connect(__ENV.URL, null, function (socket) {
    socket.on('open', function open() {
      // console.log('connected')
      let iters = 0
      socket.setInterval(function interval() {
        if (iters < 10000) {
          iters += 1
          socket.send(JSON.stringify({
            'OrderType': Math.random() < 0.5 ? "Buy" : "Sell",
            // 'OrderType': "Buy",
            'Amount': 1,
            'Price': 1 + getRandomInt(9),
            'Symbol': "AAPL",
            'TraderId': __ENV.TRADERID,
            'Password': __ENV.PASSWORD.split("")
          }));
        } else {
          exec.test.abort()
        }
      }, .01);
    });
    // socket.on('message', (data) => {
    //   console.log('Message received: ', data)
    //   // l_res = data
    // });
  });
  // console.log(l_res);
  check(res, { 'status is 101': (r) => r && r.status === 101 });
}