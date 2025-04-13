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
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import CheckIcon from '@mui/icons-material/Check';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
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
  
  // Code block component with copy button
  const CodeBlock = ({ language, value }) => {
    const codeRef = useRef(null);
    const [copied, setCopied] = useState(false);
    
    // Handle copy to clipboard
    const handleCopy = () => {
      if (codeRef.current) {
        navigator.clipboard.writeText(value);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }
    };
    
    return (
      <Box 
        sx={{ 
          my: 1,
          position: 'relative',
          border: '1px solid',
          borderColor: 'divider',
          borderRadius: '4px',
          '&:hover .copy-button': {
            opacity: 1
          }
        }}
      >
        <Tooltip title={copied ? "Copied!" : "Copy code"}>
          <IconButton
            className="copy-button"
            size="small"
            onClick={handleCopy}
            sx={{
              position: 'absolute',
              top: 8,
              right: 8,
              opacity: 0,
              transition: 'opacity 0.2s',
              bgcolor: 'rgba(0, 0, 0, 0.3)',
              color: 'white',
              '&:hover': {
                bgcolor: 'rgba(0, 0, 0, 0.5)'
              },
              zIndex: 1
            }}
          >
            {copied ? <CheckIcon fontSize="small" /> : <ContentCopyIcon fontSize="small" />}
          </IconButton>
        </Tooltip>
        <SyntaxHighlighter
          ref={codeRef}
          style={vscDarkPlus}
          language={language}
          wrapLines={true}
          customStyle={{
            borderRadius: '3px',
            margin: 0,
            fontSize: '0.85rem',
          }}
        >
          {value}
        </SyntaxHighlighter>
      </Box>
    );
  };
  
  // Custom components for markdown rendering
  const MarkdownComponents = {
    // Custom renderer for code blocks
    code: ({node, inline, className, children, ...props}) => {
      const match = /language-(\w+)/.exec(className || '');
      const language = match ? match[1] : '';
      const value = String(children).replace(/\n$/, '');
      
      return !inline && match ? (
        <CodeBlock language={language} value={value} />
      ) : (
        <code className={className} {...props} style={{ 
          backgroundColor: 'rgba(255, 255, 255, 0.1)', 
          borderRadius: '3px', 
          padding: '0.2em 0.4em',
          fontSize: '85%'
        }}>
          {children}
        </code>
      );
    },
    // Custom styling for other markdown elements
    p: ({children}) => <Typography variant="body1" sx={{ my: 1 }}>{children}</Typography>,
    h1: ({children}) => <Typography variant="h5" sx={{ mt: 2, mb: 1 }}>{children}</Typography>,
    h2: ({children}) => <Typography variant="h6" sx={{ mt: 2, mb: 1 }}>{children}</Typography>,
    h3: ({children}) => <Typography variant="subtitle1" sx={{ mt: 1.5, mb: 0.75, fontWeight: 'bold' }}>{children}</Typography>,
    h4: ({children}) => <Typography variant="subtitle2" sx={{ mt: 1.5, mb: 0.75, fontWeight: 'bold' }}>{children}</Typography>,
    ul: ({children}) => <Box component="ul" sx={{ pl: 2, my: 1 }}>{children}</Box>,
    ol: ({children}) => <Box component="ol" sx={{ pl: 2, my: 1 }}>{children}</Box>,
    li: ({children}) => <Box component="li" sx={{ my: 0.5 }}>{children}</Box>,
    blockquote: ({children}) => (
      <Box 
        component="blockquote" 
        sx={{ 
          borderLeft: '4px solid', 
          borderColor: 'primary.main',
          pl: 2, 
          py: 0.5,
          my: 1,
          bgcolor: 'rgba(144, 202, 249, 0.08)',
          borderRadius: '2px'
        }}
      >
        {children}
      </Box>
    ),
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
          {isUser ? (
            <Typography variant="body1" sx={{ whiteSpace: 'pre-wrap' }}>
              {message.content}
            </Typography>
          ) : (
            <ReactMarkdown components={MarkdownComponents}>
              {message.content}
            </ReactMarkdown>
          )}
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
