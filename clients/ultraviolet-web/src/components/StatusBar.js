import React from 'react';
import { Box, Typography } from '@mui/material';
import FiberManualRecordIcon from '@mui/icons-material/FiberManualRecord';

/**
 * StatusBar component displays the connection status
 * @param {Object} props - Component props
 * @param {boolean} props.connected - Whether the connection is established
 */
function StatusBar({ connected }) {
  return (
    <Box 
      sx={{ 
        display: 'flex', 
        alignItems: 'center',
      }}
    >
      <FiberManualRecordIcon 
        sx={{ 
          fontSize: 12, 
          mr: 1, 
          color: connected ? 'success.main' : 'error.main',
          animation: connected ? 'none' : 'pulse 2s infinite'
        }} 
      />
      <Typography variant="body2">
        {connected ? 'Connected' : 'Disconnected'}
      </Typography>
      
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
