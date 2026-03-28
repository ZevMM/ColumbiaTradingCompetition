import { useState, useEffect, useRef } from 'react';

function App() {
  const [time, setTime] = useState(0);
  const [isRunning, setIsRunning] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const wsRef = useRef(null);

  useEffect(() => {
    function connect() {
      const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      // In production, admin server WebSocket is proxied at /ws by nginx.
      // In dev, connect directly to admin server.
      // Auto-detect: if served under /timer, WS is at /timer/ws; otherwise /ws
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
      justifyContent: "center",
      alignItems: "center",
      backgroundColor: "rgb(10, 10, 18)",
    }}>
      <div style={{ fontFamily: "IBM Plex Sans", color: "white", textAlign: "center" }}>
        <h2 style={{ fontSize: "10vh", color: statusColor }}>
          Time Remaining: {minutes}:{String(seconds).padStart(2, "0")}
          {statusText}
        </h2>
      </div>
    </div>
  );
}

export default App;
