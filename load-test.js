#!/usr/bin/env node
//
// Load test for the trading exchange.
// Simulates N traders connecting via WebSocket, placing orders, and cancelling.
//
// Usage:
//   npm install ws
//   node load-test.js                          # 60 traders against localhost
//   node load-test.js --url wss://exchange.columbia.trade --traders 60
//   node load-test.js --url ws://localhost:8080 --traders 30 --ramp-delay 50
//
// Flags:
//   --url <ws url>          WebSocket base URL (default: ws://localhost:8080)
//   --traders <n>           Number of traders to simulate (default: 60)
//   --ramp-delay <ms>       Delay between each trader connecting (default: 100)
//   --order-interval <ms>   Base interval between orders per trader (default: 1500)
//   --duration <s>          How long to run after all traders connect (default: 120)
//   --market-data           Also open a market_data/ws connection per trader
//   --no-trade              Connect only, don't send orders (test connection stability)

const WebSocket = require("ws");

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
function flag(name, fallback) {
  const i = args.indexOf(`--${name}`);
  if (i === -1) return fallback;
  return args[i + 1];
}
function hasFlag(name) {
  return args.includes(`--${name}`);
}

const BASE_URL = flag("url", "ws://localhost:8080");
const NUM_TRADERS = parseInt(flag("traders", "60"), 10);
const RAMP_DELAY = parseInt(flag("ramp-delay", "100"), 10);
const ORDER_INTERVAL = parseInt(flag("order-interval", "1500"), 10);
const DURATION = parseInt(flag("duration", "120"), 10);
const OPEN_MARKET_DATA = hasFlag("market-data");
const NO_TRADE = hasFlag("no-trade");

const ASSETS = ["AD", "TS", "TT"];

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------
const stats = {
  connected: 0,
  disconnected: 0,
  ordersSent: 0,
  ordersConfirmed: 0,
  ordersRejected: 0,
  cancelsSent: 0,
  tradesBroadcast: 0,
  errors: 0,
  marketDataConnected: 0,
  marketDataDisconnected: 0,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function passwordArray(pw) {
  return pw.split("");
}

function log(traderId, msg) {
  const ts = new Date().toISOString().slice(11, 23);
  console.log(`[${ts}] ${traderId}: ${msg}`);
}

// ---------------------------------------------------------------------------
// Trader simulation
// ---------------------------------------------------------------------------
async function simulateTrader(traderId, password) {
  const url = `${BASE_URL}/orders/ws`;
  const protocol = `${traderId}|${password}`;
  const pwArray = passwordArray(password);

  let ws;
  try {
    ws = new WebSocket(url, [protocol]);
  } catch (err) {
    log(traderId, `FAILED TO CREATE WS: ${err.message}`);
    stats.errors++;
    return;
  }

  const activeOrders = []; // track order IDs for cancellation
  let gameStarted = false;

  return new Promise((resolve) => {
    let orderLoop = null;

    ws.on("open", () => {
      stats.connected++;
      log(traderId, `connected (${stats.connected}/${NUM_TRADERS})`);
    });

    ws.on("message", (raw) => {
      let msg;
      try {
        msg = JSON.parse(raw);
      } catch {
        return;
      }

      if ("GameStartedMessage" in msg) {
        gameStarted = true;
        log(traderId, "game started");
      }

      if ("OrderConfirmMessage" in msg) {
        stats.ordersConfirmed++;
        const orderId = msg.OrderConfirmMessage.order_info.order_id;
        activeOrders.push(msg.OrderConfirmMessage.order_info);
      }

      if ("OrderPlaceErrorMessage" in msg) {
        stats.ordersRejected++;
      }

      if ("CancelConfirmMessage" in msg) {
        const cancelledId = msg.CancelConfirmMessage.order_info.order_id;
        const idx = activeOrders.findIndex((o) => o.order_id === cancelledId);
        if (idx !== -1) activeOrders.splice(idx, 1);
      }

      if ("TradeOccurredMessage" in msg) {
        stats.tradesBroadcast++;
      }

      if ("Error" in msg) {
        stats.errors++;
        log(traderId, `ERROR: ${msg.Error}`);
      }
    });

    ws.on("close", (code, reason) => {
      stats.disconnected++;
      log(
        traderId,
        `DISCONNECTED code=${code} reason=${reason || "none"} (${stats.disconnected} total disconnects)`
      );
      if (orderLoop) clearInterval(orderLoop);
      resolve();
    });

    ws.on("error", (err) => {
      stats.errors++;
      log(traderId, `WS ERROR: ${err.message}`);
    });

    // Start sending orders after connection
    if (!NO_TRADE) {
      ws.on("open", () => {
        // Randomize interval per trader so they don't all fire at once
        const jitter = Math.floor(Math.random() * ORDER_INTERVAL);
        orderLoop = setInterval(() => {
          if (ws.readyState !== WebSocket.OPEN) return;

          // 20% chance to cancel an existing order if we have any
          if (activeOrders.length > 0 && Math.random() < 0.2) {
            const order =
              activeOrders[Math.floor(Math.random() * activeOrders.length)];
            const cancelMsg = {
              MessageType: "CancelRequest",
              OrderId: order.order_id,
              TraderId: traderId,
              Price: order.price,
              Symbol: order.symbol,
              Side: order.order_type,
              Password: pwArray,
            };
            ws.send(JSON.stringify(cancelMsg));
            stats.cancelsSent++;
            return;
          }

          // Place a new order
          const asset = ASSETS[Math.floor(Math.random() * ASSETS.length)];
          const side = Math.random() > 0.5 ? "Buy" : "Sell";
          const price = Math.floor(30 + Math.random() * 40); // 30-70 cents
          const amount = Math.floor(1 + Math.random() * 10); // 1-10 shares

          const orderMsg = {
            MessageType: "OrderRequest",
            OrderType: side,
            Amount: amount,
            Price: price,
            Symbol: asset,
            TraderId: traderId,
            Password: pwArray,
          };
          ws.send(JSON.stringify(orderMsg));
          stats.ordersSent++;
        }, ORDER_INTERVAL + jitter);
      });
    }

    // Auto-close after duration
    setTimeout(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
      resolve();
    }, (DURATION + 30) * 1000); // extra 30s buffer for ramp-up
  });
}

// ---------------------------------------------------------------------------
// Market data connection (optional, tests broadcast fan-out)
// ---------------------------------------------------------------------------
function openMarketData(label) {
  const url = `${BASE_URL}/market_data/ws`;
  const ws = new WebSocket(url);
  ws.on("open", () => {
    stats.marketDataConnected++;
  });
  ws.on("close", () => {
    stats.marketDataDisconnected++;
    log(label, "MARKET DATA DISCONNECTED");
  });
  ws.on("error", (err) => {
    stats.errors++;
  });
  return ws;
}

// ---------------------------------------------------------------------------
// Stats printer
// ---------------------------------------------------------------------------
function printStats() {
  console.log("\n--- Load Test Stats ---");
  console.log(`  Connected:          ${stats.connected}/${NUM_TRADERS}`);
  console.log(`  Disconnected:       ${stats.disconnected}`);
  console.log(`  Orders sent:        ${stats.ordersSent}`);
  console.log(`  Orders confirmed:   ${stats.ordersConfirmed}`);
  console.log(`  Orders rejected:    ${stats.ordersRejected}`);
  console.log(`  Cancels sent:       ${stats.cancelsSent}`);
  console.log(`  Trades broadcast:   ${stats.tradesBroadcast}`);
  console.log(`  Errors:             ${stats.errors}`);
  if (OPEN_MARKET_DATA) {
    console.log(`  MD connected:       ${stats.marketDataConnected}`);
    console.log(`  MD disconnected:    ${stats.marketDataDisconnected}`);
  }
  console.log("------------------------\n");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  console.log(`\nLoad test: ${NUM_TRADERS} traders → ${BASE_URL}`);
  console.log(
    `  ramp-delay=${RAMP_DELAY}ms  order-interval=${ORDER_INTERVAL}ms  duration=${DURATION}s`
  );
  console.log(
    `  market-data=${OPEN_MARKET_DATA}  no-trade=${NO_TRADE}\n`
  );

  const traders = [];
  const mdConnections = [];

  // Print stats every 10 seconds
  const statsInterval = setInterval(printStats, 10000);

  for (let i = 1; i <= NUM_TRADERS; i++) {
    const traderId = `trader${i}`;
    const password = String(i).padStart(4, "0");

    traders.push(simulateTrader(traderId, password));

    if (OPEN_MARKET_DATA) {
      mdConnections.push(openMarketData(`md-${traderId}`));
    }

    // Stagger connections
    if (i < NUM_TRADERS) await sleep(RAMP_DELAY);
  }

  console.log(`\nAll ${NUM_TRADERS} traders launched. Running for ${DURATION}s...\n`);

  // Wait for duration then wind down
  await sleep(DURATION * 1000);

  clearInterval(statsInterval);
  printStats();

  // Close any remaining market data connections
  for (const ws of mdConnections) {
    if (ws.readyState === WebSocket.OPEN) ws.close();
  }

  console.log("Load test complete.");
  process.exit(0);
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
