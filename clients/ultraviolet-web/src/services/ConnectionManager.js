/**
 * ConnectionManager handles WebSocket communication with the UV Service
 */
class ConnectionManager {
  /**
   * Create a new ConnectionManager
   * @param {string} url - WebSocket URL to connect to, or null to use same host
   */
  constructor(url = null) {
    // If no URL provided, connect to the same host that served the page
    this.url = url || `ws://${window.location.host}/ws`;
    this.socket = null;
    this.connected = false;
    this.pendingRequests = new Map();
    this.connectionListeners = [];
  }

  /**
   * Connect to the WebSocket server
   * @returns {Promise} Resolves when connected, rejects on error
   */
  connect() {
    return new Promise((resolve, reject) => {
      this.socket = new WebSocket(this.url);
      
      this.socket.onopen = () => {
        this.connected = true;
        this.notifyConnectionListeners(true);
        resolve();
      };
      
      this.socket.onclose = () => {
        this.connected = false;
        this.notifyConnectionListeners(false);
      };
      
      this.socket.onerror = (error) => {
        // Make sure we update the connection status on error
        this.connected = false;
        this.notifyConnectionListeners(false);
        reject(error);
      };
      
      this.socket.onmessage = (event) => {
        this.handleMessage(event.data);
      };
    });
  }

  /**
   * Check if the WebSocket is currently connected
   * @returns {boolean} True if connected, false otherwise
   */
  isConnected() {
    return (
      this.socket && 
      this.socket.readyState === WebSocket.OPEN
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
   * Notify all listeners of connection state changes
   * @param {boolean} connected - Whether the connection is established
   */
  notifyConnectionListeners(connected) {
    this.connectionListeners.forEach(listener => listener(connected));
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
