/**
 * ConnectionManager handles WebSocket communication with the UV Service
 */
class ConnectionManager {
  /**
   * Create a new ConnectionManager
   * @param {string} url - WebSocket URL to connect to, or null to use same host
   * @param {Object} options - Configuration options
   */
  constructor(url = null, options = {}) {
    // If no URL provided, connect to the same host that served the page
    this.url = url || `ws://${window.location.host}/ws`;
    this.socket = null;
    this.connected = false;
    this.pendingRequests = new Map();
    this.connectionListeners = [];
    this.stateListeners = [];
    
    // Reconnection configuration
    this.maxRetries = options.maxRetries || 10;
    this.retryDelay = options.retryDelay || 1000; // Start with 1 second
    this.maxRetryDelay = options.maxRetryDelay || 30000; // Max 30 seconds
    this.currentRetries = 0;
    this.reconnectTimer = null;
    this.connectionState = 'disconnected'; // disconnected, connecting, connected, reconnecting, failed
    this.shouldReconnect = true; // Flag to control reconnection behavior
  }

  /**
   * Connect to the WebSocket server
   * @param {boolean} isReconnect - Whether this is a reconnection attempt
   * @returns {Promise} Resolves when connected, rejects on error
   */
  connect(isReconnect = false) {
    // Clear any existing reconnection timer
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    // Update connection state
    this.setConnectionState('connecting');

    return new Promise((resolve, reject) => {
      try {
        this.socket = new WebSocket(this.url);
        
        this.socket.onopen = () => {
          console.log('WebSocket connected');
          this.connected = true;
          this.currentRetries = 0; // Reset retry counter on successful connection
          this.setConnectionState('connected');
          this.notifyConnectionListeners(true);
          resolve();
        };
        
        this.socket.onclose = (event) => {
          console.log('WebSocket closed:', event.code, event.reason);
          this.connected = false;
          this.notifyConnectionListeners(false);
          
          // Only attempt reconnection if we should reconnect and this wasn't a manual close
          if (this.shouldReconnect && event.code !== 1000) {
            this.handleDisconnection();
          } else {
            this.setConnectionState('disconnected');
          }
        };
        
        this.socket.onerror = (error) => {
          console.error('WebSocket error:', error);
          this.connected = false;
          this.notifyConnectionListeners(false);
          
          // If this is the initial connection attempt, reject the promise
          if (!isReconnect) {
            reject(error);
          }
        };
        
        this.socket.onmessage = (event) => {
          this.handleMessage(event.data);
        };
      } catch (error) {
        console.error('Failed to create WebSocket:', error);
        this.setConnectionState('failed');
        reject(error);
      }
    });
  }

  /**
   * Check if the WebSocket is currently connected
   * @returns {boolean} True if connected, false otherwise
   */
  isConnected() {
    return (
      this.socket && 
      (this.socket.readyState === WebSocket.OPEN)
    );
  }

  /**
   * Send a wavefront and return a promise that resolves with the response
   * @param {string} prism - Prism ID in namespace:name format
   * @param {string} frequency - Frequency (method) to call
   * @param {object} input - Input data for the wavefront
   * @returns {Promise} Resolves with response data, rejects on error
   */
  sendWavefront(prism, frequency, input) {
    if (!this.isConnected()) {
      return Promise.reject(new Error('Not connected to server'));
    }
    
    const id = this.generateUUID();
    const message = {
      Wavefront: {
        id,
        prism,
        frequency,
        input
      }
    };
    
    return new Promise((resolve, reject) => {
      const responseData = [];
      
      this.pendingRequests.set(id, {
        onPhoton: (data) => {
          responseData.push(data);
        },
        onTrap: (error) => {
          this.pendingRequests.delete(id);
          if (error) {
            reject(error);
          } else {
            resolve(responseData);
          }
        }
      });
      
      this.socket.send(JSON.stringify(message));
    });
  }

  /**
   * Send a wavefront with streaming response, calling a callback for each token
   * @param {string} prism - Prism ID in namespace:name format
   * @param {string} frequency - Frequency (method) to call
   * @param {object} input - Input data for the wavefront
   * @param {function} onToken - Callback function called with each token as it arrives
   * @returns {Promise} Resolves with complete response data array when finished, rejects on error
   */
  sendStreamingWavefront(prism, frequency, input, onToken) {
    if (!this.isConnected()) {
      return Promise.reject(new Error('Not connected to server'));
    }
    
    const id = this.generateUUID();
    const message = {
      Wavefront: {
        id,
        prism,
        frequency,
        input
      }
    };
    
    return new Promise((resolve, reject) => {
      const responseData = [];
      
      this.pendingRequests.set(id, {
        onPhoton: (data) => {
          responseData.push(data);
          if (onToken && typeof onToken === 'function') {
            onToken(data);
          }
        },
        onTrap: (error) => {
          this.pendingRequests.delete(id);
          if (error) {
            reject(error);
          } else {
            resolve(responseData);
          }
        }
      });
      
      this.socket.send(JSON.stringify(message));
    });
  }

  /**
   * Handle incoming WebSocket messages
   * @param {string} data - Message data
   */
  handleMessage(data) {
    try {
      const message = JSON.parse(data);
      
      if (message.Photon) {
        const { id, data } = message.Photon;
        const request = this.pendingRequests.get(id);
        if (request) {
          request.onPhoton(data);
        }
      } else if (message.Trap) {
        const { id, error } = message.Trap;
        const request = this.pendingRequests.get(id);
        if (request) {
          request.onTrap(error);
        }
      }
    } catch (error) {
      console.error('Failed to parse message:', error);
    }
  }

  /**
   * Handle disconnection and attempt reconnection
   */
  handleDisconnection() {
    if (this.currentRetries < this.maxRetries) {
      this.scheduleReconnect();
    } else {
      console.log('Max reconnection attempts reached');
      this.setConnectionState('failed');
    }
  }

  /**
   * Schedule a reconnection attempt with exponential backoff
   */
  scheduleReconnect() {
    const delay = Math.min(
      this.retryDelay * Math.pow(2, this.currentRetries), 
      this.maxRetryDelay
    );
    
    console.log(`Scheduling reconnection attempt ${this.currentRetries + 1}/${this.maxRetries} in ${delay}ms`);
    this.setConnectionState('reconnecting');
    
    this.reconnectTimer = setTimeout(() => {
      this.attemptReconnect();
    }, delay);
  }

  /**
   * Attempt to reconnect
   */
  async attemptReconnect() {
    this.currentRetries++;
    console.log(`Reconnection attempt ${this.currentRetries}/${this.maxRetries}`);
    
    try {
      await this.connect(true);
      console.log('Reconnection successful');
    } catch (error) {
      console.error('Reconnection failed:', error);
      // The connect method will handle scheduling the next attempt via onclose
    }
  }

  /**
   * Manually trigger reconnection (resets retry counter)
   * @returns {Promise} Resolves when connected, rejects on error
   */
  reconnect() {
    console.log('Manual reconnection triggered');
    this.currentRetries = 0; // Reset retry counter for manual reconnection
    this.shouldReconnect = true; // Ensure reconnection is enabled
    
    // Close existing connection if any
    if (this.socket) {
      this.shouldReconnect = false; // Temporarily disable auto-reconnect
      this.socket.close(1000, 'Manual reconnection');
      this.shouldReconnect = true; // Re-enable auto-reconnect
    }
    
    return this.connect();
  }

  /**
   * Disconnect and disable automatic reconnection
   */
  disconnect() {
    console.log('Manual disconnection');
    this.shouldReconnect = false;
    
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    
    if (this.socket) {
      this.socket.close(1000, 'Manual disconnection');
    }
    
    this.setConnectionState('disconnected');
  }

  /**
   * Get current connection state
   * @returns {string} Current connection state
   */
  getConnectionState() {
    return this.connectionState;
  }

  /**
   * Set connection state and notify listeners
   * @param {string} state - New connection state
   */
  setConnectionState(state) {
    if (this.connectionState !== state) {
      console.log(`Connection state changed: ${this.connectionState} -> ${state}`);
      this.connectionState = state;
      this.notifyStateListeners(state);
    }
  }

  /**
   * Add connection state listener
   * @param {function} listener - Function to call when connection state changes
   * @returns {function} Function to remove the listener
   */
  addConnectionListener(listener) {
    this.connectionListeners.push(listener);
    return () => {
      this.connectionListeners = this.connectionListeners.filter(l => l !== listener);
    };
  }

  /**
   * Add connection state listener (for detailed state changes)
   * @param {function} listener - Function to call when connection state changes
   * @returns {function} Function to remove the listener
   */
  addStateListener(listener) {
    this.stateListeners.push(listener);
    return () => {
      this.stateListeners = this.stateListeners.filter(l => l !== listener);
    };
  }

  /**
   * Notify all listeners of connection state changes
   * @param {boolean} connected - Whether the connection is established
   */
  notifyConnectionListeners(connected) {
    this.connectionListeners.forEach(listener => listener(connected));
  }

  /**
   * Notify all listeners of detailed state changes
   * @param {string} state - Current connection state
   */
  notifyStateListeners(state) {
    this.stateListeners.forEach(listener => listener(state));
  }

  /**
   * Generate a UUID for request correlation
   * @returns {string} UUID
   */
  generateUUID() {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
      const r = Math.random() * 16 | 0;
      const v = c === 'x' ? r : (r & 0x3 | 0x8);
      return v.toString(16);
    });
  }
}

export default ConnectionManager;
