import React, { useState, useEffect, useRef } from 'react';
import { ThemeProvider, createTheme, CssBaseline } from '@mui/material';
import Navigation from './Navigation';
import PrismExplorer from './PrismExplorer';
import StatusBar from './StatusBar';
import ConnectionManager from '../services/ConnectionManager';
import PrismDiscovery from '../services/PrismDiscovery';
import '../App.css';

// Create a theme
const theme = createTheme({
  palette: {
    primary: {
      main: '#2196f3', // Blue
    },
    secondary: {
      main: '#ff9800', // Orange
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
      <div className="app">
        <header className="app-header">
          <h1>Ultraviolet Web Client</h1>
          <StatusBar connected={connected} />
        </header>
        
        <div className="app-content">
          <Navigation 
            prisms={prisms} 
            loading={loading}
            onSelectPrism={setSelectedPrism} 
            selectedPrism={selectedPrism}
            connected={connected}
          />
          
          <PrismExplorer 
            prismId={selectedPrism}
            prismDiscovery={prismDiscoveryRef.current}
            connectionManager={connectionManagerRef.current}
          />
        </div>
        
        {error && (
          <div className="error-message">
            {error}
            <div className="error-actions">
              <button onClick={() => setError(null)}>Dismiss</button>
              {!connected && (
                <button onClick={handleRetryConnection}>Retry Connection</button>
              )}
            </div>
          </div>
        )}
      </div>
    </ThemeProvider>
  );
}

export default App;
