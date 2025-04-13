import React, { useState, useEffect, useRef } from 'react';
import { 
  Box, 
  Button, 
  IconButton,
  ThemeProvider, 
  ToggleButton, 
  ToggleButtonGroup, 
  Tooltip, 
  createTheme, 
  CssBaseline, 
  Typography 
} from '@mui/material';
import AppsIcon from '@mui/icons-material/Apps';
import ChatIcon from '@mui/icons-material/Chat';
import ChatView from './ChatView';
import Navigation from './Navigation';
import PrismExplorer from './PrismExplorer';
import StatusBar from './StatusBar';
import ConnectionManager from '../services/ConnectionManager';
import PrismDiscovery from '../services/PrismDiscovery';
import '../App.css';

// Create a dark theme
const theme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#90caf9', // Lighter blue for dark mode
    },
    secondary: {
      main: '#ffb74d', // Lighter orange for dark mode
    },
    background: {
      default: '#121212',
      paper: '#1e1e1e',
    },
  },
  typography: {
    fontFamily: [
      '-apple-system',
      'BlinkMacSystemFont',
      '"Segoe UI"',
      'Roboto',
      'Oxygen',
      'Ubuntu',
      'Cantarell',
      '"Open Sans"',
      '"Helvetica Neue"',
      'sans-serif',
    ].join(','),
  },
  components: {
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
        },
      },
    },
  },
});

/**
 * Main application component
 */
function App() {
  const [connected, setConnected] = useState(false);
  const [prisms, setPrisms] = useState([]);
  const [selectedPrism, setSelectedPrism] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [currentView, setCurrentView] = useState('prisms'); // 'prisms' or 'chat'
  
  // Use refs to maintain service instances across renders
  const connectionManagerRef = useRef(null);
  const prismDiscoveryRef = useRef(null);
  
  // Initialize services on mount
  useEffect(() => {
    connectionManagerRef.current = new ConnectionManager('ws://localhost:3000/ws');
    prismDiscoveryRef.current = new PrismDiscovery(connectionManagerRef.current);
    
    // Set up connection listener
    const removeListener = connectionManagerRef.current.addConnectionListener(setConnected);
    
    // Explicitly set initial connection state to false
    setConnected(false);
    
    // Connect to server
    connectionManagerRef.current.connect()
      .then(() => loadPrisms())
      .catch(err => {
        setError(`Connection error: ${err.message}`);
        // Ensure connected state is false on error
        setConnected(false);
      });
      
    return () => {
      // Clean up
      removeListener();
    };
  }, []);
  
  // Periodically check connection status
  useEffect(() => {
    // Check connection status every 2 seconds
    const intervalId = setInterval(() => {
      if (connectionManagerRef.current) {
        const isConnected = connectionManagerRef.current.isConnected();
        
        // If connection state changed from disconnected to connected, reload prisms
        if (isConnected && !connected) {
          loadPrisms();
        }
        
        setConnected(isConnected);
      }
    }, 2000);
    
    return () => clearInterval(intervalId);
  }, [connected]); // eslint-disable-line react-hooks/exhaustive-deps
  
  // Load prisms when connected
  const loadPrisms = async () => {
    if (!connectionManagerRef.current || !prismDiscoveryRef.current) {
      return;
    }
    
    setLoading(true);
    try {
      const prismList = await prismDiscoveryRef.current.listPrisms();
      setPrisms(prismList);
      setError(null);
    } catch (err) {
      setError(`Failed to load prisms: ${err.message}`);
    } finally {
      setLoading(false);
    }
  };
  
  // Retry connection
  const handleRetryConnection = () => {
    if (!connectionManagerRef.current) {
      return;
    }
    
    connectionManagerRef.current.connect()
      .then(() => loadPrisms())
      .catch(err => setError(`Connection error: ${err.message}`));
  };
  
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Box 
        sx={{ 
          display: 'flex', 
          flexDirection: 'column', 
          height: '100vh'
        }}
      >
        <Box 
          component="header" 
          sx={{ 
            display: 'flex', 
            justifyContent: 'space-between', 
            alignItems: 'center', 
            p: 2, 
            bgcolor: 'background.paper', 
            borderBottom: 1, 
            borderColor: 'divider',
            boxShadow: 1
          }}
        >
          <Typography variant="h5">Ultraviolet Web Client</Typography>
          
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
            <ToggleButtonGroup
              value={currentView}
              exclusive
              onChange={(e, newView) => newView && setCurrentView(newView)}
              aria-label="view selector"
              size="small"
            >
              <ToggleButton value="prisms" aria-label="prisms view">
                <Tooltip title="Prisms Explorer">
                  <AppsIcon />
                </Tooltip>
              </ToggleButton>
              <ToggleButton value="chat" aria-label="chat view">
                <Tooltip title="AI Chat">
                  <ChatIcon />
                </Tooltip>
              </ToggleButton>
            </ToggleButtonGroup>
            
            <StatusBar connected={connected} />
          </Box>
        </Box>
        
        <Box sx={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
          {currentView === 'prisms' ? (
            <>
              <Navigation 
                prisms={prisms} 
                loading={loading}
                onSelectPrism={setSelectedPrism} 
                selectedPrism={selectedPrism}
                connected={connected}
              />
              
              <Box sx={{ flex: 1, overflow: 'auto' }}>
                <PrismExplorer 
                  prismId={selectedPrism}
                  prismDiscovery={prismDiscoveryRef.current}
                  connectionManager={connectionManagerRef.current}
                />
              </Box>
            </>
          ) : (
            <ChatView 
              connectionManager={connectionManagerRef.current}
            />
          )}
        </Box>
        
        {error && (
          <Box 
            sx={{ 
              position: 'fixed',
              bottom: 16,
              right: 16,
              p: 2,
              bgcolor: 'error.main',
              color: 'error.contrastText',
              borderRadius: 1,
              boxShadow: 3,
              maxWidth: 400,
              zIndex: 1000
            }}
          >
            <Typography variant="body2" sx={{ mb: 1 }}>{error}</Typography>
            <Box sx={{ display: 'flex', justifyContent: 'flex-end', gap: 1 }}>
              <Button 
                size="small" 
                variant="outlined" 
                color="inherit" 
                onClick={() => setError(null)}
              >
                Dismiss
              </Button>
              {!connected && (
                <Button 
                  size="small" 
                  variant="outlined" 
                  color="inherit" 
                  onClick={handleRetryConnection}
                >
                  Retry Connection
                </Button>
              )}
            </Box>
          </Box>
        )}
      </Box>
    </ThemeProvider>
  );
}

export default App;
