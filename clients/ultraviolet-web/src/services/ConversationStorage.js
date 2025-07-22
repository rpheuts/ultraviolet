/**
 * ConversationStorage manages conversation persistence using localStorage
 * Supports multiple browser tabs with shared conversation data
 */
class ConversationStorage {
  constructor() {
    this.SESSIONS_KEY = 'uv_chat_sessions';
    this.MESSAGES_PREFIX = 'uv_chat_messages_';
    this.ACTIVE_SESSIONS_KEY = 'uv_active_sessions';
    this.MAX_SESSIONS = 50; // Limit to prevent localStorage bloat
    
    // Initialize storage if needed
    this.initializeStorage();
    
    // Listen for storage changes from other tabs
    this.setupCrossTabSync();
  }

  /**
   * Initialize localStorage structure if it doesn't exist
   */
  initializeStorage() {
    if (!localStorage.getItem(this.SESSIONS_KEY)) {
      localStorage.setItem(this.SESSIONS_KEY, JSON.stringify({}));
    }
    if (!localStorage.getItem(this.ACTIVE_SESSIONS_KEY)) {
      localStorage.setItem(this.ACTIVE_SESSIONS_KEY, JSON.stringify([]));
    }
  }

  /**
   * Generate a unique session ID
   * @returns {string} UUID-like session ID
   */
  generateSessionId() {
    return 'session_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
  }

  /**
   * Generate a unique message ID
   * @returns {string} UUID-like message ID
   */
  generateMessageId() {
    return 'msg_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
  }

  /**
   * Create a new conversation session
   * @param {string} model - AI model being used
   * @param {boolean} agentMode - Whether agent mode was used
   * @param {string} title - Optional custom title
   * @returns {Object} New session object
   */
  createNewSession(model = 'q-claude-4-sonnet', agentMode = true, title = null) {
    const sessionId = this.generateSessionId();
    const now = new Date().toISOString();
    
    const session = {
      id: sessionId,
      title: title || 'New Conversation',
      createdAt: now,
      lastUpdatedAt: now,
      model: model,
      agentMode: agentMode,
      messageCount: 0,
      isActive: true
    };

    // Add to sessions index
    const sessions = this.getAllSessions();
    sessions[sessionId] = session;
    localStorage.setItem(this.SESSIONS_KEY, JSON.stringify(sessions));

    // Initialize empty messages array
    localStorage.setItem(this.MESSAGES_PREFIX + sessionId, JSON.stringify([]));

    // Add to active sessions
    this.addActiveSession(sessionId);

    // Cleanup old sessions if we exceed the limit
    this.cleanupOldSessions();

    console.log('Created new conversation session:', sessionId);
    return session;
  }

  /**
   * Get all conversation sessions
   * @returns {Object} Sessions indexed by ID
   */
  getAllSessions() {
    try {
      const sessions = localStorage.getItem(this.SESSIONS_KEY);
      return sessions ? JSON.parse(sessions) : {};
    } catch (error) {
      console.error('Error loading sessions:', error);
      return {};
    }
  }

  /**
   * Get a specific session by ID
   * @param {string} sessionId - Session ID
   * @returns {Object|null} Session object or null if not found
   */
  getSession(sessionId) {
    const sessions = this.getAllSessions();
    return sessions[sessionId] || null;
  }

  /**
   * Update session metadata
   * @param {string} sessionId - Session ID
   * @param {Object} updates - Fields to update
   */
  updateSessionMetadata(sessionId, updates) {
    const sessions = this.getAllSessions();
    if (sessions[sessionId]) {
      sessions[sessionId] = {
        ...sessions[sessionId],
        ...updates,
        lastUpdatedAt: new Date().toISOString()
      };
      localStorage.setItem(this.SESSIONS_KEY, JSON.stringify(sessions));
    }
  }

  /**
   * Delete a conversation session and its messages
   * @param {string} sessionId - Session ID to delete
   */
  deleteSession(sessionId) {
    // Remove from sessions index
    const sessions = this.getAllSessions();
    delete sessions[sessionId];
    localStorage.setItem(this.SESSIONS_KEY, JSON.stringify(sessions));

    // Remove messages
    localStorage.removeItem(this.MESSAGES_PREFIX + sessionId);

    // Remove from active sessions
    this.removeActiveSession(sessionId);

    console.log('Deleted conversation session:', sessionId);
  }

  /**
   * Save a message to a conversation
   * @param {string} sessionId - Session ID
   * @param {Object} message - Message object
   */
  saveMessage(sessionId, message) {
    const messages = this.getMessages(sessionId);
    
    // Ensure message has required fields
    const fullMessage = {
      id: message.id || this.generateMessageId(),
      timestamp: message.timestamp || new Date().toISOString(),
      ...message
    };

    messages.push(fullMessage);
    localStorage.setItem(this.MESSAGES_PREFIX + sessionId, JSON.stringify(messages));

    // Update session metadata
    this.updateSessionMetadata(sessionId, {
      messageCount: messages.length,
      title: this.generateTitleFromMessages(messages)
    });

    return fullMessage;
  }

  /**
   * Get all messages for a conversation
   * @param {string} sessionId - Session ID
   * @returns {Array} Array of message objects
   */
  getMessages(sessionId) {
    try {
      const messages = localStorage.getItem(this.MESSAGES_PREFIX + sessionId);
      return messages ? JSON.parse(messages) : [];
    } catch (error) {
      console.error('Error loading messages for session:', sessionId, error);
      return [];
    }
  }

  /**
   * Delete a specific message from a conversation
   * @param {string} sessionId - Session ID
   * @param {string} messageId - Message ID to delete
   */
  deleteMessage(sessionId, messageId) {
    const messages = this.getMessages(sessionId);
    const filteredMessages = messages.filter(msg => msg.id !== messageId);
    localStorage.setItem(this.MESSAGES_PREFIX + sessionId, JSON.stringify(filteredMessages));
    
    // Update session metadata
    this.updateSessionMetadata(sessionId, {
      messageCount: filteredMessages.length,
      title: this.generateTitleFromMessages(filteredMessages)
    });
    
    console.log('Deleted message:', messageId, 'from session:', sessionId);
    return filteredMessages;
  }

  /**
   * Clear all messages from a conversation
   * @param {string} sessionId - Session ID
   */
  clearMessages(sessionId) {
    localStorage.setItem(this.MESSAGES_PREFIX + sessionId, JSON.stringify([]));
    this.updateSessionMetadata(sessionId, {
      messageCount: 0,
      title: 'New Conversation'
    });
  }

  /**
   * Generate a meaningful title from the first few messages
   * @param {Array} messages - Array of messages
   * @returns {string} Generated title
   */
  generateTitleFromMessages(messages) {
    if (messages.length === 0) {
      return 'New Conversation';
    }

    // Find the first user message
    const firstUserMessage = messages.find(msg => msg.role === 'user');
    if (!firstUserMessage) {
      return 'New Conversation';
    }

    // Extract first 50 characters and clean up
    let title = firstUserMessage.content.substring(0, 50).trim();
    
    // Remove newlines and extra spaces
    title = title.replace(/\s+/g, ' ');
    
    // Add ellipsis if truncated
    if (firstUserMessage.content.length > 50) {
      title += '...';
    }

    return title || 'New Conversation';
  }

  /**
   * Get recent sessions sorted by last updated
   * @param {number} limit - Maximum number of sessions to return
   * @returns {Array} Array of session objects
   */
  getRecentSessions(limit = 10) {
    const sessions = this.getAllSessions();
    return Object.values(sessions)
      .sort((a, b) => new Date(b.lastUpdatedAt) - new Date(a.lastUpdatedAt))
      .slice(0, limit);
  }

  /**
   * Add session to active sessions list (for cross-tab awareness)
   * @param {string} sessionId - Session ID
   */
  addActiveSession(sessionId) {
    try {
      const activeSessions = JSON.parse(localStorage.getItem(this.ACTIVE_SESSIONS_KEY) || '[]');
      if (!activeSessions.includes(sessionId)) {
        activeSessions.push(sessionId);
        localStorage.setItem(this.ACTIVE_SESSIONS_KEY, JSON.stringify(activeSessions));
      }
    } catch (error) {
      console.error('Error updating active sessions:', error);
    }
  }

  /**
   * Remove session from active sessions list
   * @param {string} sessionId - Session ID
   */
  removeActiveSession(sessionId) {
    try {
      const activeSessions = JSON.parse(localStorage.getItem(this.ACTIVE_SESSIONS_KEY) || '[]');
      const filtered = activeSessions.filter(id => id !== sessionId);
      localStorage.setItem(this.ACTIVE_SESSIONS_KEY, JSON.stringify(filtered));
    } catch (error) {
      console.error('Error updating active sessions:', error);
    }
  }

  /**
   * Get list of currently active sessions (across all tabs)
   * @returns {Array} Array of active session IDs
   */
  getActiveSessions() {
    try {
      return JSON.parse(localStorage.getItem(this.ACTIVE_SESSIONS_KEY) || '[]');
    } catch (error) {
      console.error('Error loading active sessions:', error);
      return [];
    }
  }

  /**
   * Clean up old sessions to prevent localStorage bloat
   */
  cleanupOldSessions() {
    const sessions = this.getAllSessions();
    const sessionList = Object.values(sessions);

    if (sessionList.length <= this.MAX_SESSIONS) {
      return; // No cleanup needed
    }

    // Sort by last updated, keep the most recent ones
    const sortedSessions = sessionList.sort(
      (a, b) => new Date(b.lastUpdatedAt) - new Date(a.lastUpdatedAt)
    );

    // Delete the oldest sessions
    const sessionsToDelete = sortedSessions.slice(this.MAX_SESSIONS);
    sessionsToDelete.forEach(session => {
      this.deleteSession(session.id);
    });

    console.log(`Cleaned up ${sessionsToDelete.length} old conversation sessions`);
  }

  /**
   * Export a session as JSON
   * @param {string} sessionId - Session ID
   * @returns {Object} Exportable session data
   */
  exportSession(sessionId) {
    const session = this.getSession(sessionId);
    const messages = this.getMessages(sessionId);
    
    if (!session) {
      throw new Error(`Session ${sessionId} not found`);
    }

    return {
      session,
      messages,
      exportedAt: new Date().toISOString(),
      version: '1.0'
    };
  }

  /**
   * Import a session from exported JSON
   * @param {Object} sessionData - Exported session data
   * @returns {string} New session ID
   */
  importSession(sessionData) {
    const newSessionId = this.generateSessionId();
    const now = new Date().toISOString();

    // Create new session with imported data
    const session = {
      ...sessionData.session,
      id: newSessionId,
      createdAt: now,
      lastUpdatedAt: now,
      title: sessionData.session.title + ' (Imported)'
    };

    // Save session
    const sessions = this.getAllSessions();
    sessions[newSessionId] = session;
    localStorage.setItem(this.SESSIONS_KEY, JSON.stringify(sessions));

    // Save messages
    localStorage.setItem(this.MESSAGES_PREFIX + newSessionId, JSON.stringify(sessionData.messages));

    console.log('Imported conversation session:', newSessionId);
    return newSessionId;
  }

  /**
   * Set up cross-tab synchronization using storage events
   */
  setupCrossTabSync() {
    window.addEventListener('storage', (event) => {
      // Only handle our storage keys
      if (event.key === this.SESSIONS_KEY || 
          event.key === this.ACTIVE_SESSIONS_KEY ||
          event.key?.startsWith(this.MESSAGES_PREFIX)) {
        
        // Emit custom event for components to listen to
        window.dispatchEvent(new CustomEvent('conversationStorageChange', {
          detail: {
            key: event.key,
            oldValue: event.oldValue,
            newValue: event.newValue
          }
        }));
      }
    });
  }

  /**
   * Get storage usage statistics
   * @returns {Object} Storage usage info
   */
  getStorageStats() {
    const sessions = this.getAllSessions();
    const sessionCount = Object.keys(sessions).length;
    
    let totalMessages = 0;
    let totalSize = 0;

    // Calculate total messages and approximate size
    Object.keys(sessions).forEach(sessionId => {
      const messages = this.getMessages(sessionId);
      totalMessages += messages.length;
      totalSize += JSON.stringify(messages).length;
    });

    // Add sessions index size
    totalSize += JSON.stringify(sessions).length;

    return {
      sessionCount,
      totalMessages,
      approximateSizeKB: Math.round(totalSize / 1024),
      maxSessions: this.MAX_SESSIONS
    };
  }
}

export default ConversationStorage;
