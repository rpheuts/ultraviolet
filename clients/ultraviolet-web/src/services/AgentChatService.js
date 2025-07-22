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
    this.backend = 'bedrock'; // Default backend (bedrock or q)
  }

  /**
   * Set the AI model to use (optional override)
   * @param {string} model - Model ID
   */
  setModel(model) {
    this.model = model;
  }

  /**
   * Set the backend preference based on the selected prism
   * @param {string} prismId - Prism ID (e.g., 'core:bedrock', 'core:q')
   */
  setPrism(prismId) {
    // Agent prism ID never changes - it's always 'ai:agent'
    // We only extract the backend preference from the selected prism
    if (prismId === 'core:q') {
      this.backend = 'q';
    } else if (prismId === 'core:ollama') {
      this.backend = 'ollama';
    }
    else {
      this.backend = 'bedrock';
    }
  }

  /**
   * Format conversation history into a prompt for the agent
   * @param {Array} messages - Array of message objects with role and content
   * @param {Array} contextFiles - Array of file objects with name and content
   * @returns {string} Formatted prompt
   */
  formatConversationPrompt(messages, contextFiles = []) {
    if (!messages || messages.length === 0) {
      return '';
    }

    // Format conversation messages, including action results
    let prompt = messages.map(msg => {
      if (msg.role === 'user') {
        return `User: ${msg.content}`;
      } else if (msg.role === 'assistant') {
        return `Assistant: ${msg.content}`;
      } else if (msg.role === 'system' && msg.type === 'action_result') {
        // Format action results for context
        const { action, success, data, error } = msg.content;
        let actionSummary = `Action: ${action.prism} ${action.frequency} (${action.description})`;
        
        if (success && data) {
          // Include full data without truncation
          const dataStr = JSON.stringify(data);
          actionSummary += ` -> Success: ${dataStr}`;
        } else if (!success && error) {
          actionSummary += ` -> Error: ${error}`;
        }
        
        return actionSummary;
      }
      return '';
    }).filter(line => line.trim()).join('\n\n');
    
    // Add file context if available
    if (contextFiles && contextFiles.length > 0) {
      prompt += '\n\n--- Context Files ---\n';
      
      contextFiles.forEach(file => {
        prompt += `\n### File: ${file.name} ###\n${file.content}\n`;
      });
      
      prompt += '\n--- End Context Files ---\n';
    }
    
    return prompt;
  }

  /**
   * Send a message to the AI agent and get structured event responses
   * @param {Array} conversationHistory - Array of previous messages
   * @param {string} userMessage - User's natural language request
   * @param {Array} contextFiles - Array of file objects with name and content
   * @param {function} onEvent - Callback for each event as it arrives
   * @returns {Promise} Resolves when agent completes the workflow
   */
  sendMessage(conversationHistory, userMessage, contextFiles, onEvent) {
    // Format only the previous conversation history (not including current user message)
    // The current user message will be included via the prompt template's {user_prompt} placeholder
    const prompt = this.formatConversationPrompt(conversationHistory, contextFiles);

    // Prepare the input for the agent prism
    const input = {
      prompt,
      include_examples: true,
      backend: this.backend
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
    // Handle raw token/usage data (not structured agent events)
    if (eventData && eventData.token !== undefined) {
      // This is a token stream event, possibly with usage data
      if (eventData.usage) {
        return {
          type: 'usage',
          usage: eventData.usage,
          display: 'usage'
        };
      } else if (eventData.token) {
        // Regular token for streaming
        return {
          type: 'token',
          token: eventData.token,
          display: 'token'
        };
      }
      return null;
    }

    // Handle structured agent events
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
