import { useState, useEffect, useRef, useCallback} from 'react'
import './App.css'
import Login from './Login/Login'
import Console from './Console/Console'
import WaitScreen from './Views/WaitScreen'
import EndScreen from './Views/EndScreen'
import ErrorPopup from './Error'

const addr = import.meta.env.VITE_WS_URL || `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/orders/ws`
const MAX_PRICE_HISTORY = 5000

function App() {
  const [user, setUser] = useState(null)
  const [err, setErr] = useState(null)
  const [ws, setWs] = useState(null)
  const [game, setGame] = useState(null)
  const [account, setAccount] = useState(null)
  const [trades, setTrades] = useState([])  // own fill history
  const [state, setState] = useState(0)
  const gameref = useRef(game)
  const accountref = useRef(account)
  const kickedref = useRef(false)
  // incase order fill comes before order confirm — stores list of {amount, price} per order_id
  const tmpFillRef = useRef({})
  const [final_score, setFinalScore] = useState(0)
  const [retry, setRetry] = useState(0)
  const pendingGameUpdates = useRef([])
  const gameFrame = useRef(null)
  const feedSequence = useRef(0)
  const telemetryRef = useRef({sequence: 0, lastMessageAt: null})
  const [connectionStatus, setConnectionStatus] = useState('disconnected')
  const [telemetry, setTelemetry] = useState(telemetryRef.current)
  const queueGameUpdate = useCallback((update) => {
    pendingGameUpdates.current.push(update)
    if (gameFrame.current !== null) return
    gameFrame.current = requestAnimationFrame(() => {
      const updates = pendingGameUpdates.current.splice(0)
      gameFrame.current = null
      setGame(current => updates.reduce((next, apply) => apply(next), current))
    })
  }, [])

  useEffect(() => () => {
    if (gameFrame.current !== null) cancelAnimationFrame(gameFrame.current)
  }, [])

  useEffect(() => {
    const id = setInterval(() => setTelemetry({...telemetryRef.current}), 500)
    return () => clearInterval(id)
  }, [])

  useEffect(() => {
    if (user) {
        kickedref.current = false;
        feedSequence.current = 0;
        telemetryRef.current = {sequence: 0, lastMessageAt: null}
        setConnectionStatus('connecting')
        let newws = new WebSocket(addr, [`${user.uid}|${user.pwd}`]);
        newws.onerror = (error) => {
          console.error("WebSocket error:", error);
          setUser(null);
          setWs(null);
          setErr("Connection failed — check username/password, or try again in a few seconds");
          setConnectionStatus('disconnected')
          setState(0);
        };
        newws.onopen = () => setConnectionStatus('connected')
        newws.onclose = () => {
          setConnectionStatus(kickedref.current ? 'disconnected' : 'reconnecting')
          if (kickedref.current) {
            // Don't reconnect — we were kicked by another session
            return;
          }
          setRetry(retry + 1);
        };
        newws.onmessage = function(e) {
          telemetryRef.current.lastMessageAt = Date.now()
          const raw = JSON.parse(e.data)
          const messages = Array.isArray(raw) ? raw : [raw]
          for (const msg of messages) {
            let [type, body] = Object.entries(msg)[0]
            if (typeof body?.sequence === 'number') {
              if (body.sequence <= feedSequence.current) continue
              if (feedSequence.current > 0 && body.sequence !== feedSequence.current + 1) {
                newws.send(JSON.stringify({MessageType: "GameStateRequest"}))
              }
              feedSequence.current = body.sequence
              telemetryRef.current.sequence = body.sequence
            }
            switch (type) {
            case "GameStartedMessage":
              setState(1)
              break;
            case "GameEndMessage": {
              setState(2)

              const urpl = Object.entries(accountref.current.asset_balances).reduce(
                (s, [k,v]) => {
                    return (s + (100 * v * (gameref.current[k].price_history?.at(-1)?.[1] ?? 0)) / (100 + v))
                }, 0
              )
              const net_value = urpl + accountref.current.cents_balance;

              setFinalScore(net_value.toFixed(0));
              break;
            }
            case "GameState":
              pendingGameUpdates.current = []
              setGame(body)
              break;
            case "AccountInfo":
              setAccount(body)
              break;
            case "TradeOccurredMessage": {
              let {amount, symbol, resting_side, price, time, sequence} = body
              queueGameUpdate(prevGame => {
                const sideKey = resting_side == "Buy" ? 'buy_side' : 'sell_side'
                const oldSide = prevGame[symbol][sideKey]
                const newSide = {...oldSide}
                if (sequence === undefined) {
                  const cur = oldSide[price] || 0
                  if (cur - amount <= 0) delete newSide[price]
                  else newSide[price] = cur - amount
                }
                return {
                  ...prevGame,
                  [symbol]: {
                    ...prevGame[symbol],
                    [sideKey]: newSide,
                    price_history: [...prevGame[symbol].price_history.slice(-(MAX_PRICE_HISTORY - 1)), [time ?? Math.floor(Date.now() / 1000), price, amount]]
                  }
                }
              });
              break;
            }
            case "NewRestingOrderMessage": {
              let {side, amount, symbol, price} = body
              queueGameUpdate(prevGame => {
                const sideKey = side == "Buy" ? 'buy_side' : 'sell_side'
                const oldSide = prevGame[symbol][sideKey]
                return {
                  ...prevGame,
                  [symbol]: {
                    ...prevGame[symbol],
                    [sideKey]: {
                      ...oldSide,
                      [price]: (oldSide[price] || 0) + amount
                    }
                  }
                }
              });
              break;
            }
            case "BookLevelUpdate": {
              const {side, symbol, price, quantity} = body
              queueGameUpdate(prevGame => {
                const sideKey = side == "Buy" ? 'buy_side' : 'sell_side'
                const newSide = {...prevGame[symbol][sideKey]}
                if (quantity === 0) delete newSide[price]
                else newSide[price] = quantity
                return {...prevGame, [symbol]: {...prevGame[symbol], [sideKey]: newSide}}
              })
              break;
            }
            case "OrderPlaceErrorMessage":
              setErr(body.error_details)
              break;
            case "OrderConfirmMessage": {
              body = body.order_info
              setAccount(prevAccount => {
                let newaccount = {...prevAccount}
                let {price, order_type, amount, symbol, order_id} = body;

                if (order_type == "Buy") {
                    newaccount.net_cents_balance -= price * amount
                }
                else {
                    newaccount.net_asset_balances[body.symbol] -= amount
                }

                if (order_id in tmpFillRef.current) {
                    const fills = tmpFillRef.current[order_id];
                    const newTrades = [];
                    for (const f of fills) {
                        amount -= f.amount;
                        if (order_type == "Buy") {
                            newaccount.cents_balance -= f.price * f.amount
                            newaccount.net_cents_balance += (price - f.price) * f.amount
                            newaccount.asset_balances[symbol] += f.amount
                            newaccount.net_asset_balances[symbol] += f.amount
                        } else {
                            newaccount.cents_balance += f.price * f.amount
                            newaccount.net_cents_balance += f.price * f.amount
                            newaccount.asset_balances[symbol] -= f.amount
                        }
                        newTrades.push({ts: Date.now(), symbol, side: order_type, amount: f.amount, price: f.price});
                    }
                    setTrades(prev => [...newTrades.reverse(), ...prev].slice(0, 200))
                    delete tmpFillRef.current[order_id];
                }

                if (amount > 0) {
                    body.amount = amount
                    newaccount.active_orders.push(body)
                }

                return newaccount;
              });
              break;
            }
            case "OrderFillMessage": {
              let {order_id, amount_filled, price} = body
              setAccount(prevAccount => {
                  let newaccount = {...prevAccount}
                  let idx = newaccount.active_orders.findIndex(
                      (e) => e.order_id == order_id
                  )
                  if (idx == -1) {
                      if (order_id in tmpFillRef.current) {
                          tmpFillRef.current[order_id].push({amount: amount_filled, price})
                      } else {
                          tmpFillRef.current[order_id] = [{amount: amount_filled, price}]
                      }
                      return prevAccount; // Return unchanged if order not found
                  }
                  
                  let {order_type, symbol, amount, price: limit_price} = newaccount.active_orders[idx]
                  if (order_type == "Buy") {
                      newaccount.cents_balance -= price * amount_filled
                      newaccount.net_cents_balance += (limit_price - price) * amount_filled
                      newaccount.asset_balances[symbol] += amount_filled
                      newaccount.net_asset_balances[symbol] += amount_filled
                  } else {
                      newaccount.cents_balance += price * amount_filled
                      newaccount.net_cents_balance += price * amount_filled
                      newaccount.asset_balances[symbol] -= amount_filled
                  }

                  setTrades(prev => [{ts: Date.now(), symbol, side: order_type, amount: amount_filled, price}, ...prev].slice(0, 200))

                  if (amount == amount_filled) {
                      newaccount.active_orders.splice(idx, 1);
                  } else {
                      newaccount.active_orders[idx].amount -= amount_filled;
                  }

                  return newaccount;
              });
              break;
            }
            case "CancelConfirmMessage": {
              body = body.order_info
              setAccount(prevAccount => {
                let newaccount = {...prevAccount}
                let idx = newaccount.active_orders.findIndex(
                    (e) => e.order_id == body.order_id
                )
                let {order_type, symbol, amount, price} = newaccount.active_orders[idx]
                if (order_type == "Buy") {
                    newaccount.net_cents_balance += price * amount
                } else {
                    newaccount.net_asset_balances[symbol] += amount
                }
                newaccount.active_orders.splice(idx, 1)
                return newaccount
              });
              break; 
            }
            case "CancelErrorMessage":
              setErr(body.error_details)
              break;
            case "Error":
              if (typeof body === "string" && body.toLowerCase().includes("another session")) {
                kickedref.current = true;
                newws.close();
                setUser(null);
                setWs(null);
                setErr("Disconnected — another device logged in with your account.");
                setState(0);
              } else {
                setErr(body)
              }
              break;
            case "CancelOccurredMessage": {
              let {symbol, price, side, amount} = body
              queueGameUpdate(prevGame => {
                const sideKey = side == "Buy" ? 'buy_side' : 'sell_side'
                const oldSide = prevGame[symbol][sideKey]
                const cur = oldSide[price] || 0
                const newSide = {...oldSide}
                if (cur - amount <= 0) {
                  delete newSide[price]
                } else {
                  newSide[price] = cur - amount
                }
                return {
                  ...prevGame,
                  [symbol]: {
                    ...prevGame[symbol],
                    [sideKey]: newSide
                  }
                }
              });
              break;
            }
              
          }
        }
      };
    
        setWs(newws);
      }
    }, [user, retry, queueGameUpdate])

    useEffect(() => {gameref.current = game}, [game])
    useEffect(() => {accountref.current = account}, [account])
    const clearError = useCallback(() => setErr(null), [])
  
  return (
    <>
      {err && <ErrorPopup message={err} clearError={clearError} />}
      {state === 2 && <EndScreen final_score={final_score} />}
      {ws && user && state === 1 && <Console ws={ws} user={user} game={game} account={account} trades={trades} connectionStatus={connectionStatus} telemetry={telemetry} />}
      {ws && user && state === 0 && <WaitScreen />}
      {(!ws || !user) && <Login user={user} setUser={setUser} setWs={setWs}/>}
    </>
  )
}

export default App
