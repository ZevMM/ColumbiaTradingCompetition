import { useState, useEffect } from 'react';

function App() {
  const [time, setTime] = useState(0); // Time in seconds
  const [isRunning, setIsRunning] = useState(false);
  const [isPaused, setIsPaused] = useState(false); // To track pause/unpause state
  const [inputTime, setInputTime] = useState(""); // For user input in minutes

  useEffect(() => {
    let timer;
    if (isRunning && !isPaused && time > 0) {
      timer = setInterval(() => {
        setTime((prevTime) => prevTime - 1);
      }, 1000);
    } else if (time === 0 && isRunning) {
      setIsRunning(false);
      sendHttpRequest("end_game"); // Send HTTP request on end
    }
    return () => clearInterval(timer); // Cleanup interval on unmount or stop
  }, [isRunning, isPaused, time]);

  const handleStart = () => {
    const parsedTime = parseFloat(inputTime); // Parse input as a float for fractional minutes
    if (!isNaN(parsedTime) && parsedTime > 0) {
      setTime(Math.round(parsedTime * 60)); // Convert minutes to seconds
      setIsRunning(true);
      setIsPaused(false); // Ensure the timer starts in an unpaused state
      sendHttpRequest("start_game"); // Send HTTP request on start
    } else {
      alert("Please enter a valid time in minutes.");
    }
  };

  const handlePauseUnpause = () => {
    sendHttpRequest(isPaused ? "start_game" : "end_game");
    setIsPaused((prevPaused) => !prevPaused); // Toggle pause/unpause state
  };

  const handleClear = () => {
    setIsRunning(false);
    setIsPaused(false);
    setTime(0);
    setInputTime(""); // Clear the input field
    sendHttpRequest("end_game"); // Send HTTP request on clear
  };

  const sendHttpRequest = async (action) => {
    try {
      const response = await fetch(`https://trading-competition-148005249496.us-east4.run.app/${action}`);
      if (!response.ok) {
        console.error("Failed to send HTTP request");
      }
    } catch (error) {
      console.error("Error sending HTTP request:", error);
    }
  };

  return (
    <div style={{ height: "100vh", display: "flex", justifyContent: "center", alignItems: "center", backgroundColor: "rgb(10, 10, 18)" }}>
      <div style={{ fontFamily: "IBM Plex Sans", color: "white" }}>
        <h2 style={{ marginTop: "20px", textAlign: "center", fontSize: "10vh" }}>
          Time Remaining: {Math.floor(time / 60)}:{String(time % 60).padStart(2, "0")}
        </h2>
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center" }}>
          <div>
          <input
            type="number"
            placeholder="Set time (minutes)"
            value={inputTime}
            onChange={(e) => setInputTime(e.target.value)}
            disabled={isRunning}
            style={{ padding: "10px", fontSize: "16px", marginRight: "10px" }}
          />
          {!isRunning ? (
            <button
              onClick={handleStart}
              style={{ padding: "10px 20px", fontSize: "16px", marginRight: "10px" }}
            >
              Start
            </button>
          ) : (
            <button
              onClick={handlePauseUnpause}
              style={{ padding: "10px 20px", fontSize: "16px", marginRight: "10px" }}
            >
              {isPaused ? "Continue" : "Pause"}
            </button>
          )}
            <button
              onClick={handleClear}
              style={{ padding: "10px 20px", fontSize: "16px" }}
            >
              Clear
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;