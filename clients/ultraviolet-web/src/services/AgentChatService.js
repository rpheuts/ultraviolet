/**
 * AgentChatService manages communication with the AI agent prism
 * Handles structured event streams and agent orchestration
 */
class AgentChatService {
  /**
   * Create a new AgentChatService
   * @param {ConnectionManager} connectionManager - ConnectionManager instance
   */
  constructor(connectionManager) {
    this.connectionManager = connectionManager;
    this.prismId = 'ai:agent'; // Agent prism ID
    this.model = null; // Optional model override
  }

  /**
   * Set the AI model to use (optional override)
   * @param {string} model - Model ID
   */
  setModel(model) {
    this.model = model;
  }

  /**
   * Set the prism backend to use (e.g., 'core:bedrock', 'core:q')
   * @param {string} prismId - Prism ID
   */
  setPrism(prismId) {
    this.prismId = prismId;
  }

  /**
   * Send a message to the AI agent and get structured event responses
   * @param {string} userMessage - User's natural language request
   * @param {Array} contextFiles - Array of file objects with name and content
   * @param {function} onEvent - Callback for each event as it arrives
   * @returns {Promise} Resolves when agent completes the workflow
   */
  sendMessage(userMessage, contextFiles, onEvent) {
    // Build the prompt with context files if provided
    let prompt = userMessage;
    
    if (contextFiles && contextFiles.length > 0) {
      prompt += '\n\n--- Context Files ---\n';
      
      contextFiles.forEach(file => {
        prompt += `\n### File: ${file.name} ###\n${file.content}\n`;
      });
      
      prompt += '\n--- End Context Files ---\n';
    }

    // Prepare the input for the agent prism
    const input = {
      prompt,
      include_examples: true
    };
    
    // Add model if specified
    if (this.model) {
      input.model = this.model;
    }
    
    // Send the request with structured event streaming
    return this.connectionManager.sendStreamingWavefront(
      this.prismId,
      'execute',
      input,
      onEvent
    );
  }

  /**
   * Process an agent event and determine its display type
   * @param {Object} eventData - Raw event data from agent
   * @returns {Object} Processed event with display information
   */
  processEvent(eventData) {
    if (!eventData || !eventData.type) {
      return null;
    }

    switch (eventData.type) {
      case 'ai_response':
        return {
          type: 'ai_response',
          content: eventData.content,
          display: 'message'
        };

      case 'ai_reasoning':
        return {
          type: 'ai_reasoning',
          content: eventData.content,
          display: 'reasoning'
        };

      case 'ai_progress':
        return {
          type: 'ai_progress',
          content: eventData.content,
          display: 'progress'
        };

      case 'action_output':
        return {
          type: 'action_output',
          action: eventData.action,
          success: eventData.success,
          data: eventData.data,
          error: eventData.error,
          display: 'action_result'
        };

      case 'complete':
        return {
          type: 'complete',
          display: 'completion'
        };

      default:
        console.warn('Unknown agent event type:', eventData.type);
        return null;
    }
  }
}

export default AgentChatService;
