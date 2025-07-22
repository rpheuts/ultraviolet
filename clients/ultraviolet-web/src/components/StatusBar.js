import React from 'react';
import { Box, Typography, IconButton, Tooltip, CircularProgress } from '@mui/material';
import FiberManualRecordIcon from '@mui/icons-material/FiberManualRecord';
import RefreshIcon from '@mui/icons-material/Refresh';

/**
 * StatusBar component displays the connection status
 * @param {Object} props - Component props
 * @param {boolean} props.connected - Whether the connection is established
 * @param {string} props.connectionState - Detailed connection state
 * @param {function} props.onReconnect - Function to call when reconnect is requested
 */
function StatusBar({ connected, connectionState = 'disconnected', onReconnect }) {
  // Determine display text and color based on connection state
  const getStatusInfo = () => {
    switch (connectionState) {
      case 'connecting':
        return { text: 'Connecting...', color: 'warning.main', showSpinner: true };
      case 'connected':
        return { text: 'Connected', color: 'success.main', showSpinner: false };
      case 'reconnecting':
        return { text: 'Reconnecting...', color: 'warning.main', showSpinner: true };
      case 'failed':
        return { text: 'Connection Failed', color: 'error.main', showSpinner: false };
      case 'disconnected':
      default:
        return { text: 'Disconnected', color: 'error.main', showSpinner: false };
    }
  };

  const statusInfo = getStatusInfo();
  const canReconnect = !connected && connectionState !== 'connecting' && connectionState !== 'reconnecting';

  return (
    <Box 
      sx={{ 
        display: 'flex', 
        alignItems: 'center',
        gap: 1
      }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center' }}>
        {statusInfo.showSpinner ? (
          <CircularProgress 
            size={12} 
            sx={{ 
              mr: 1,
              color: statusInfo.color
            }} 
          />
        ) : (
          <FiberManualRecordIcon 
            sx={{ 
              fontSize: 12, 
              mr: 1, 
              color: statusInfo.color,
              animation: !connected && !statusInfo.showSpinner ? 'pulse 2s infinite' : 'none'
            }} 
          />
        )}
        <Typography variant="body2" sx={{ color: statusInfo.color }}>
          {statusInfo.text}
        </Typography>
      </Box>
      
      {/* Reconnect button - only show when disconnected/failed and not currently connecting */}
      {canReconnect && onReconnect && (
        <Tooltip title="Click to reconnect">
          <IconButton
            size="small"
            onClick={onReconnect}
            sx={{
              color: 'text.secondary',
              '&:hover': {
                color: 'primary.main'
              }
            }}
          >
            <RefreshIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      )}
      
      {/* Add keyframes for pulsing effect */}
      <style jsx="true">{`
        @keyframes pulse {
          0% { opacity: 1; }
          50% { opacity: 0.5; }
          100% { opacity: 1; }
        }
      `}</style>
    </Box>
  );
}

export default StatusBar;
