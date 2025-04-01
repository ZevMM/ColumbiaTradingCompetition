import React, { useEffect } from 'react';

function ErrorPopup({ message, clearError }) {
  useEffect(() => {
    if (message) {
      const timer = setTimeout(() => {
        clearError(); // Clear the error after 3 seconds
      }, 3000);

      return () => clearTimeout(timer); // Cleanup the timer on unmount or when the message changes
    }
  }, [message, clearError]);

  if (!message) return null;

  return (
    <div className="error-popup">
      {message}
    </div>
  );
}

export default ErrorPopup;