/**
 * ChatService manages conversation state and communication with AI prisms (bedrock, q, etc.)
 */
class ChatService {
  /**
   * Create a new ChatService
   * @param {ConnectionManager} connectionManager - ConnectionManager instance
   */
  constructor(connectionManager) {
    this.connectionManager = connectionManager;
    this.prismId = 'core:bedrock'; // Default to Bedrock prism
    this.model = null; // Default model (null means use server default)
    this.maxTokens = 4096; // Default max tokens
  }

  /**
   * Set the AI model to use
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
   * Set the maximum number of tokens for responses
   * @param {number} maxTokens - Maximum tokens
   */
  setMaxTokens(maxTokens) {
    this.maxTokens = maxTokens;
  }

  /**
   * Format conversation history into a prompt for the AI
   * @param {Array} messages - Array of message objects with role and content
   * @param {Array} contextFiles - Array of file objects with name and content
   * @returns {string} Formatted prompt
   */
  formatConversationPrompt(messages, contextFiles = []) {
    if (!messages || messages.length === 0) {
      return '';
    }

    // Format conversation messages
    let prompt = messages.map(msg => {
      const role = msg.role === 'user' ? 'User' : 'Assistant';
      return `${role}: ${msg.content}`;
    }).join('\n\n');
    
    // Add file context if available
    if (contextFiles && contextFiles.length > 0) {
      prompt += '\n\n--- Context Files ---\n';
      
      contextFiles.forEach(file => {
        prompt += `\n### File: ${file.name} ###\n${file.content}\n`;
      });
      
      prompt += '\n--- End Context Files ---\n';
    }
    
    // Add final prompt for the assistant
    prompt += '\n\nAssistant:';
    
    return prompt;
  }

  /**
   * Send a message to the AI and get a streaming response
   * @param {Array} conversationHistory - Array of previous messages
   * @param {string} userMessage - New user message
   * @param {Array} contextFiles - Array of file objects with name and content
   * @param {function} onToken - Callback for each token as it arrives
   * @param {function} onUsage - Optional callback for token usage information
   * @returns {Promise} Resolves with complete response when finished
   */
  sendMessage(conversationHistory, userMessage, contextFiles, onToken, onUsage) {
    // Create a new conversation history with the user message
    const messages = [
      ...conversationHistory,
      { role: 'user', content: userMessage }
    ];
    
    // Format the conversation history into a prompt with file context
    const prompt = this.formatConversationPrompt(messages, contextFiles);
    
    // Prepare the input for the bedrock prism
    const input = {
      prompt,
      max_tokens: this.maxTokens
    };
    
    // Add model if specified
    if (this.model) {
      input.model = this.model;
    }
    
    // Send the request with streaming response
    return this.connectionManager.sendStreamingWavefront(
      this.prismId,
      'invoke_stream',
      input,
      (data) => {
        // Handle token data
        if (data && data.token !== undefined) {
          onToken(data);
        }
        
        // Handle usage data
        if (data && data.usage && onUsage) {
          onUsage(data.usage);
        }
      }
    );
  }
}

export default ChatService;
