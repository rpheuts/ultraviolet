import React, { useState, useEffect, useRef } from 'react';
import { 
  Box, 
  Button, 
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
  const [connectionState, setConnectionState] = useState('disconnected');
  const [prisms, setPrisms] = useState([]);
  const [selectedPrism, setSelectedPrism] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [currentView, setCurrentView] = useState('chat'); // 'prisms' or 'chat'
  
  // Use refs to maintain service instances across renders
  const connectionManagerRef = useRef(null);
  const prismDiscoveryRef = useRef(null);
  
  // Initialize services on mount
  useEffect(() => {
    connectionManagerRef.current = new ConnectionManager();
    prismDiscoveryRef.current = new PrismDiscovery(connectionManagerRef.current);
    
    // Set up connection listeners
    const removeConnectionListener = connectionManagerRef.current.addConnectionListener((isConnected) => {
      setConnected(isConnected);
      // Load prisms when connection is established
      if (isConnected) {
        loadPrisms();
      }
    });
    
    const removeStateListener = connectionManagerRef.current.addStateListener((state) => {
      setConnectionState(state);
    });
    
    // Set initial states
    setConnected(false);
    setConnectionState('disconnected');
    
    // Connect to server
    connectionManagerRef.current.connect()
      .catch(err => {
        setError(`Connection error: ${err.message}`);
      });
      
    return () => {
      // Clean up listeners
      removeConnectionListener();
      removeStateListener();
      
      // Disconnect when component unmounts
      if (connectionManagerRef.current) {
        connectionManagerRef.current.disconnect();
      }
    };
  }, []);
  
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
  
  // Handle manual reconnection
  const handleReconnect = async () => {
    if (!connectionManagerRef.current) {
      return;
    }
    
    try {
      await connectionManagerRef.current.reconnect();
      // loadPrisms will be called automatically via the connection listener
    } catch (err) {
      setError(`Reconnection failed: ${err.message}`);
    }
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
            
            <StatusBar 
              connected={connected} 
              connectionState={connectionState}
              onReconnect={handleReconnect}
            />
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
                  onClick={handleReconnect}
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
