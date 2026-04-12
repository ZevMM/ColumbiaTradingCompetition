import { useState, useEffect, useRef } from 'react';

function App() {
  const [time, setTime] = useState(0);
  const [isRunning, setIsRunning] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [showBoard, setShowBoard] = useState(false);
  const [leaders, setLeaders] = useState([]);
  const wsRef = useRef(null);

  useEffect(() => {
    function connect() {
      const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const basePath = window.location.pathname.startsWith('/timer') ? '/timer' : '';
      const wsUrl = import.meta.env.VITE_ADMIN_WS_URL || `${proto}//${window.location.host}${basePath}/ws`;
      const ws = new WebSocket(wsUrl);

      ws.onmessage = (e) => {
        const msg = JSON.parse(e.data);
        switch (msg.type) {
          case 'timer_state':
          case 'timer_tick':
            setTime(msg.remaining || 0);
            setIsRunning(msg.running);
            setIsPaused(msg.paused);
            break;
          case 'game_started':
            setTime(msg.remaining || msg.duration || 0);
            setIsRunning(true);
            setIsPaused(false);
            break;
          case 'game_paused':
            setIsPaused(true);
            break;
          case 'game_resumed':
            setIsPaused(false);
            break;
          case 'game_ended':
            setTime(0);
            setIsRunning(false);
            setIsPaused(false);
            break;
        }
      };

      ws.onclose = () => {
        setTimeout(connect, 2000);
      };

      ws.onerror = () => {
        ws.close();
      };

      wsRef.current = ws;
    }

    connect();

    return () => {
      if (wsRef.current) wsRef.current.close();
    };
  }, []);

  useEffect(() => {
    if (!isRunning) return;
    const id = setInterval(async () => {
      try {
        const res = await fetch('/api/score');
        const json = await res.json();
        if (json.ok && Array.isArray(json.data)) {
          setLeaders(json.data.slice(0, 5));
        }
      } catch (_) {}
    }, 5000);
    return () => clearInterval(id);
  }, [isRunning]);

  const minutes = Math.floor(time / 60);
  const seconds = time % 60;

  let statusText = '';
  let statusColor = '#888';
  if (isRunning && !isPaused) {
    statusColor = '#6ef';
  } else if (isPaused) {
    statusText = ' (PAUSED)';
    statusColor = '#fa3';
  }

  return (
    <div style={{
      height: "100vh",
      display: "flex",
      flexDirection: "column",
      justifyContent: "center",
      alignItems: "center",
      backgroundColor: "rgb(10, 10, 18)",
      fontFamily: "IBM Plex Sans",
      color: "white",
      position: "relative",
    }}>
      <h2 style={{ fontSize: "10vh", color: statusColor, margin: 0 }}>
        Time Remaining: {minutes}:{String(seconds).padStart(2, "0")}
        {statusText}
      </h2>

      <button
        onClick={() => setShowBoard(b => !b)}
        style={{
          position: "absolute",
          top: 24,
          right: 24,
          background: showBoard ? "#333" : "#222",
          color: "#aaa",
          border: "1px solid #444",
          borderRadius: 6,
          padding: "8px 16px",
          fontSize: "1.6vh",
          cursor: "pointer",
          fontFamily: "inherit",
        }}
      >
        {showBoard ? "Hide Leaderboard" : "Show Leaderboard"}
      </button>

      {showBoard && (
        <div style={{
          marginTop: "4vh",
          width: "min(600px, 80vw)",
          background: "rgba(255,255,255,0.04)",
          border: "1px solid #333",
          borderRadius: 10,
          overflow: "hidden",
        }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "3vh" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid #333", color: "#888" }}>
                <th style={{ padding: "12px 16px", textAlign: "left" }}>Rank</th>
                <th style={{ padding: "12px 16px", textAlign: "left" }}>Trader</th>
                <th style={{ padding: "12px 16px", textAlign: "right" }}>Score</th>
              </tr>
            </thead>
            <tbody>
              {leaders.length === 0 && (
                <tr><td colSpan={3} style={{ padding: 16, textAlign: "center", color: "#666" }}>
                  {isRunning ? "Loading..." : "Game not running"}
                </td></tr>
              )}
              {leaders.map(([name, score], i) => (
                <tr key={name} style={{
                  borderBottom: i < leaders.length - 1 ? "1px solid #222" : "none",
                  color: i === 0 ? "#ffd700" : i === 1 ? "#c0c0c0" : i === 2 ? "#cd7f32" : "#ccc",
                }}>
                  <td style={{ padding: "10px 16px" }}>{i + 1}</td>
                  <td style={{ padding: "10px 16px" }}>{name}</td>
                  <td style={{ padding: "10px 16px", textAlign: "right", fontVariantNumeric: "tabular-nums" }}>
                    {score.toLocaleString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export default App;
