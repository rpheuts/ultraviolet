import React from 'react';

/**
 * StatusBar component displays the connection status
 * @param {Object} props - Component props
 * @param {boolean} props.connected - Whether the connection is established
 */
function StatusBar({ connected }) {
  return (
    <div className={`status-bar ${connected ? 'connected' : 'disconnected'}`}>
      <span className="status-indicator"></span>
      <span className="status-text">
        {connected ? 'Connected' : 'Disconnected'}
      </span>
    </div>
  );
}

export default StatusBar;
