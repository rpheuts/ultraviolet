import React, { useState, useEffect, useRef } from 'react';
import { 
  Badge,
  Box, 
  Button, 
  Card, 
  Chip,
  CircularProgress, 
  FormControl,
  IconButton, 
  InputAdornment, 
  InputLabel,
  MenuItem,
  Paper, 
  Select,
  TextField, 
  Tooltip,
  Typography 
} from '@mui/material';
import SendIcon from '@mui/icons-material/Send';
import DeleteIcon from '@mui/icons-material/Delete';
import AttachFileIcon from '@mui/icons-material/AttachFile';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import CheckIcon from '@mui/icons-material/Check';
import PsychologyIcon from '@mui/icons-material/Psychology';
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
  const [selectedModel, setSelectedModel] = useState("us.anthropic.claude-3-7-sonnet-20250219-v1:0");
  const [showReasoning, setShowReasoning] = useState(true);
  const messagesEndRef = useRef(null);
  const chatServiceRef = useRef(null);
  
  // Initialize chat service
  useEffect(() => {
    if (connectionManager) {
      chatServiceRef.current = new ChatService(connectionManager);
      // Set default model to Claude 3.7
      chatServiceRef.current.setModel(selectedModel);
    }
  }, [connectionManager, selectedModel]);
  
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
  
  // Process message content to identify reasoning sections
  const processMessageContent = (content) => {
    // Check if the content contains reasoning tags
    if (content.includes('<reasoning>')) {
      // Split the content by reasoning tags
      const parts = content.split(/<reasoning>|<\/reasoning>/);
      
      // Create an array to hold the processed parts with their types
      const processedParts = [];
      
      // Process each part
      for (let i = 0; i < parts.length; i++) {
        if (parts[i].trim()) {
          // Even indices are regular content, odd indices are reasoning content
          const isReasoning = i % 2 !== 0;
          processedParts.push({
            content: parts[i],
            isReasoning
          });
        }
      }
      
      // Return the processed parts
      return processedParts;
    }
    
    // If no reasoning tags, return the content as a single regular part
    return [{ content, isReasoning: false }];
  };
  
  // Render a message bubble
  const renderMessage = (message, index) => {
    const isUser = message.role === 'user';
    
    if (isUser) {
      // User messages remain the same
      return (
        <Box
          key={index}
          sx={{
            display: 'flex',
            justifyContent: 'flex-end',
            mb: 2
          }}
        >
          <Paper
            elevation={1}
            sx={{
              p: 2,
              maxWidth: '80%',
              borderRadius: 2,
              bgcolor: 'primary.dark',
              color: 'primary.contrastText',
              borderTopRightRadius: 0,
              borderTopLeftRadius: 2
            }}
          >
            <Typography variant="body1" sx={{ whiteSpace: 'pre-wrap' }}>
              {message.content}
            </Typography>
          </Paper>
        </Box>
      );
    } else {
      // Process assistant messages to handle reasoning sections
      const processedContent = processMessageContent(message.content);
      
      return (
        <Box
          key={index}
          sx={{
            display: 'flex',
            justifyContent: 'flex-start',
            mb: 2
          }}
        >
          <Paper
            elevation={1}
            sx={{
              p: 2,
              maxWidth: '80%',
              borderRadius: 2,
              bgcolor: 'background.paper',
              color: 'text.primary',
              borderTopRightRadius: 2,
              borderTopLeftRadius: 0
            }}
          >
            {processedContent.map((part, partIndex) => (
              (showReasoning || !part.isReasoning) && (
                <Box 
                  key={partIndex} 
                  sx={part.isReasoning ? {
                    bgcolor: 'rgba(144, 202, 249, 0.1)',
                    borderLeft: '4px solid',
                    borderColor: 'info.main',
                    pl: 1.5,
                    py: 1,
                    my: 1,
                    borderRadius: '0 4px 4px 0',
                    position: 'relative',
                    animation: 'fadeIn 0.5s ease-in-out',
                    '@keyframes fadeIn': {
                      '0%': {
                        opacity: 0,
                        transform: 'translateY(10px)'
                      },
                      '100%': {
                        opacity: 1,
                        transform: 'translateY(0)'
                      }
                    }
                  } : {}}
                >
                  {part.isReasoning && (
                    <Typography 
                      variant="caption" 
                      sx={{ 
                        position: 'absolute',
                        top: -10,
                        left: 8,
                        bgcolor: 'info.main',
                        color: 'info.contrastText',
                        px: 1,
                        py: 0.25,
                        borderRadius: '4px 4px 0 0',
                        fontSize: '0.7rem',
                        fontWeight: 'bold'
                      }}
                    >
                      REASONING
                    </Typography>
                  )}
                  <ReactMarkdown components={MarkdownComponents}>
                    {part.content}
                  </ReactMarkdown>
                </Box>
              )
            ))}
          </Paper>
        </Box>
      );
    }
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
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
          <Typography variant="h6">AI Chat</Typography>
          <FormControl variant="outlined" size="small" sx={{ minWidth: 150 }}>
            <InputLabel>Model</InputLabel>
            <Select
              value={selectedModel}
              onChange={(e) => {
                setSelectedModel(e.target.value);
                if (chatServiceRef.current) {
                  chatServiceRef.current.setModel(e.target.value);
                }
              }}
              label="Model"
            >
              <MenuItem value="us.anthropic.claude-3-7-sonnet-20250219-v1:0">Claude 3.7</MenuItem>
              <MenuItem value="us.anthropic.claude-3-5-sonnet-20241022-v2:0">Claude 3.5</MenuItem>
              <MenuItem value="us.deepseek.r1-v1:0">DeepSeek R1</MenuItem>
              <MenuItem value="us.meta.llama3-1-405b-instruct-v1:0">Llama 3</MenuItem>
              <MenuItem value="us.amazon.nova-pro-v1:0">AWS Nova</MenuItem>
            </Select>
          </FormControl>
        </Box>
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
          <Tooltip title={showReasoning ? "Hide reasoning" : "Show reasoning"}>
            <IconButton 
              color={showReasoning ? "primary" : "inherit"}
              onClick={() => setShowReasoning(!showReasoning)}
            >
              <PsychologyIcon />
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
        onAddFiles={(newFiles) => {
          setContextFiles([...contextFiles, ...newFiles]);
          // Keep panel open after adding files
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
            multiline
            minRows={1}
            maxRows={8}
            onKeyDown={(e) => {
              // Submit on Enter, but allow Shift+Enter for newlines
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSubmit(e);
              }
            }}
            sx={{
              '& .MuiInputBase-root': {
                alignItems: 'flex-end' // Align the send button to the bottom
              }
            }}
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
