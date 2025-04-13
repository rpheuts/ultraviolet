import React, { useState, useEffect, useRef } from 'react';
import { 
  Badge,
  Box, 
  Button, 
  Card, 
  Chip,
  CircularProgress, 
  IconButton, 
  InputAdornment, 
  Paper, 
  TextField, 
  Tooltip,
  Typography 
} from '@mui/material';
import SendIcon from '@mui/icons-material/Send';
import DeleteIcon from '@mui/icons-material/Delete';
import AttachFileIcon from '@mui/icons-material/AttachFile';
import FileContextPanel from './FileContextPanel';
import ChatService from '../services/ChatService';

/**
 * ChatView component for AI chat interface
 * @param {Object} props - Component props
 * @param {Object} props.connectionManager - ConnectionManager service
 */
function ChatView({ connectionManager }) {
  const [messages, setMessages] = useState([]);
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [error, setError] = useState(null);
  const [contextFiles, setContextFiles] = useState([]);
  const [filesPanelOpen, setFilesPanelOpen] = useState(false);
  const messagesEndRef = useRef(null);
  const chatServiceRef = useRef(null);
  
  // Initialize chat service
  useEffect(() => {
    if (connectionManager) {
      chatServiceRef.current = new ChatService(connectionManager);
    }
  }, [connectionManager]);
  
  // Scroll to bottom when messages change
  useEffect(() => {
    scrollToBottom();
  }, [messages]);
  
  // Scroll to bottom of messages
  const scrollToBottom = () => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  };
  
  // Handle input change
  const handleInputChange = (e) => {
    setInputValue(e.target.value);
  };
  
  // Handle form submission
  const handleSubmit = async (e) => {
    e.preventDefault();
    
    if (!inputValue.trim() || !connectionManager || !connectionManager.isConnected()) {
      return;
    }
    
    const userMessage = inputValue.trim();
    setInputValue('');
    
    // Add user message to chat
    const updatedMessages = [
      ...messages,
      { role: 'user', content: userMessage }
    ];
    setMessages(updatedMessages);
    
    // Start AI response
    setIsTyping(true);
    setError(null);
    
    try {
      // Add an empty assistant message that will be updated with tokens
      const assistantMessageIndex = updatedMessages.length;
      setMessages([...updatedMessages, { role: 'assistant', content: '' }]);
      
      // Process each token as it arrives
      const onToken = (data) => {
        if (data && data.token) {
          setMessages(currentMessages => {
            const newMessages = [...currentMessages];
            // If this is the first token, stop the typing indicator
            if (newMessages[assistantMessageIndex].content === '') {
              setIsTyping(false);
            }
            // Append the token to the current assistant message
            newMessages[assistantMessageIndex] = {
              ...newMessages[assistantMessageIndex],
              content: newMessages[assistantMessageIndex].content + data.token
            };
            return newMessages;
          });
        }
      };
      
      // Send the message and get streaming response
      await chatServiceRef.current.sendMessage(
        updatedMessages,
        userMessage,
        contextFiles,
        onToken
      );
      
      // Ensure typing indicator is off when complete
      setIsTyping(false);
    } catch (err) {
      setError(`Error: ${err.message}`);
      setIsTyping(false);
    }
  };
  
  // Clear conversation
  const handleClearConversation = () => {
    setMessages([]);
    setError(null);
  };
  
  // Render a message bubble
  const renderMessage = (message, index) => {
    const isUser = message.role === 'user';
    
    return (
      <Box
        key={index}
        sx={{
          display: 'flex',
          justifyContent: isUser ? 'flex-end' : 'flex-start',
          mb: 2
        }}
      >
        <Paper
          elevation={1}
          sx={{
            p: 2,
            maxWidth: '80%',
            borderRadius: 2,
            bgcolor: isUser ? 'primary.dark' : 'background.paper',
            color: isUser ? 'primary.contrastText' : 'text.primary',
            borderTopRightRadius: isUser ? 0 : 2,
            borderTopLeftRadius: isUser ? 2 : 0
          }}
        >
          <Typography variant="body1" sx={{ whiteSpace: 'pre-wrap' }}>
            {message.content}
          </Typography>
        </Paper>
      </Box>
    );
  };
  
  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', width: '100%' }}>
      {/* Header */}
      <Box
        sx={{
          p: 2,
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center'
        }}
      >
        <Typography variant="h6">AI Chat</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Tooltip title="Context Files">
            <IconButton 
              color={contextFiles.length > 0 ? "primary" : "inherit"}
              onClick={() => setFilesPanelOpen(true)}
            >
              <Badge badgeContent={contextFiles.length} color="primary">
                <AttachFileIcon />
              </Badge>
            </IconButton>
          </Tooltip>
          <IconButton 
            color="inherit" 
            onClick={handleClearConversation}
            disabled={messages.length === 0}
            title="Clear conversation"
          >
            <DeleteIcon />
          </IconButton>
        </Box>
      </Box>
      
      {/* File Context Panel */}
      <FileContextPanel
        open={filesPanelOpen}
        onClose={() => setFilesPanelOpen(false)}
        files={contextFiles}
        onAddFile={(file) => {
          setContextFiles([...contextFiles, file]);
          // Keep panel open after adding a file
        }}
        onRemoveFile={(fileId) => {
          setContextFiles(contextFiles.filter(file => file.id !== fileId));
        }}
      />
      
      {/* Messages area */}
      <Box
        sx={{
          flex: 1,
          p: 2,
          overflowY: 'auto',
          display: 'flex',
          flexDirection: 'column'
        }}
      >
        {messages.length === 0 ? (
          <Box
            sx={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
              opacity: 0.7
            }}
          >
            <Typography variant="h6" color="text.secondary">
              Start a conversation
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Your messages will be sent to the AI model
            </Typography>
          </Box>
        ) : (
          messages.map(renderMessage)
        )}
        
        {/* Typing indicator */}
        {isTyping && (
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              ml: 2,
              mb: 2
            }}
          >
            <CircularProgress size={16} sx={{ mr: 1 }} />
            <Typography variant="body2" color="text.secondary">
              AI is typing...
            </Typography>
          </Box>
        )}
        
        {/* Error message */}
        {error && (
          <Box
            sx={{
              p: 2,
              mb: 2,
              bgcolor: 'error.dark',
              color: 'error.contrastText',
              borderRadius: 1
            }}
          >
            <Typography variant="body2">{error}</Typography>
          </Box>
        )}
        
        {/* Invisible element to scroll to */}
        <div ref={messagesEndRef} />
      </Box>
      
      {/* Input area */}
      <Box sx={{ p: 2, borderTop: 1, borderColor: 'divider', bgcolor: 'background.paper' }}>
        {/* File context indicator */}
        {contextFiles.length > 0 && (
          <Box sx={{ mb: 1, display: 'flex', alignItems: 'center' }}>
            <Chip
              icon={<AttachFileIcon fontSize="small" />}
              label={`${contextFiles.length} file${contextFiles.length !== 1 ? 's' : ''} in context`}
              size="small"
              color="primary"
              variant="outlined"
              onClick={() => setFilesPanelOpen(true)}
              sx={{ mr: 1 }}
            />
            <Typography variant="caption" color="text.secondary">
              Files will be included as context for the AI
            </Typography>
          </Box>
        )}
        
        {/* Message input */}
        <Box
          component="form"
          onSubmit={handleSubmit}
        >
          <TextField
            fullWidth
            variant="outlined"
            placeholder="Type your message..."
            value={inputValue}
            onChange={handleInputChange}
            disabled={!connectionManager || !connectionManager.isConnected() || isTyping}
            InputProps={{
              endAdornment: (
                <InputAdornment position="end">
                  <IconButton
                    type="submit"
                    color="primary"
                    disabled={!inputValue.trim() || !connectionManager || !connectionManager.isConnected() || isTyping}
                  >
                    <SendIcon />
                  </IconButton>
                </InputAdornment>
              )
            }}
          />
        </Box>
      </Box>
    </Box>
  );
}

export default ChatView;
