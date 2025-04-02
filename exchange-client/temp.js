import WebSocket from "ws";

const websocket = new WebSocket("ws://localhost:8080/market_data/ws");
let game;

websocket.onmessage = function(e) {

  let [type, body] = Object.entries(JSON.parse(e.data))[0]
  
  switch (type) {
    
    // handles initial orderbook dump
    case "GameState": {
      
      game = body
      
      break;
    }

    case "TradeOccurredMessage": {
      let {amount, symbol, resting_side, price, time} = body
      
      game[symbol][
        resting_side == "Buy" ?
        'buy_side_limit_levels' :
        'sell_side_limit_levels'
      ][price].total_volume -= amount;
      game[symbol].price_history.push([time, price, amount])

      if (resting_side == "Buy")
        game[symbol].current_high_buy_price = game
                                              [symbol]
                                              .buy_side_limit_levels
                                              .findLastIndex((e) => e.total_volume > 0);
      else
        game[symbol].current_low_sell_price = game
                                              [symbol]
                                              .sell_side_limit_levels
                                              .findIndex((e) => e.total_volume > 0);

      break;
    }

    case "NewRestingOrderMessage": {
      let {side, amount, symbol, price} = body

      game[symbol][
        side == "Buy" ?
        'buy_side_limit_levels' :
        'sell_side_limit_levels'
      ][price].total_volume += amount;

      if (
        side == "Buy" &&
        price > game[symbol].current_high_buy_price
      ) {
        game[symbol].current_high_buy_price = price
      }
      else if (
        side == "Sell" &&
        price < game[symbol].current_low_sell_price
      ) {
        game[symbol].current_low_sell_price = price
      }

      break;
    }

    case "CancelOccurredMessage": {
      let {symbol, price, side, amount} = body
      
      game[symbol][
        side == "Buy" ?
        'buy_side_limit_levels' :
        'sell_side_limit_levels'
      ][price].total_volume -= amount

      if (side == "Buy")
        game[symbol].current_high_buy_price = game
                                              [symbol]
                                              .buy_side_limit_levels
                                              .findLastIndex(
                                                (e) => e.total_volume > 0
                                              );
      else
        game[symbol].current_low_sell_price = game
                                              [symbol]
                                              .sell_side_limit_levels
                                              .findIndex((e) => e.total_volume > 0);

      break;
    }
  }
};