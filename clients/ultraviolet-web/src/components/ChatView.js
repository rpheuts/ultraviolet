import React, { useState, useEffect, useRef } from 'react';
import { 
  Badge,
  Box, 
  Button, 
  Card, 
  Chip,
  CircularProgress, 
  Collapse,
  FormControl,
  FormControlLabel,
  IconButton, 
  InputAdornment, 
  InputLabel,
  LinearProgress,
  MenuItem,
  Paper, 
  Select,
  Switch,
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
import SmartToyIcon from '@mui/icons-material/SmartToy';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import ErrorIcon from '@mui/icons-material/Error';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import ExpandLessIcon from '@mui/icons-material/ExpandLess';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import FileContextPanel from './FileContextPanel';
import ChatService from '../services/ChatService';
import AgentChatService from '../services/AgentChatService';

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
  const [selectedModelOption, setSelectedModelOption] = useState("bedrock-claude-4-sonnet");
  const [showReasoning, setShowReasoning] = useState(false);
  const [agentMode, setAgentMode] = useState(false);
  const [agentProgress, setAgentProgress] = useState([]);
  const [expandedActions, setExpandedActions] = useState({});
  const messagesEndRef = useRef(null);
  const chatServiceRef = useRef(null);
  const agentServiceRef = useRef(null);

  // Model options with backend routing
  const MODEL_OPTIONS = [
    // Bedrock Models
    { 
      id: 'bedrock-claude-4-sonnet', 
      label: 'Bedrock - Claude Sonnet 4.0', 
      backend: 'bedrock', 
      model: 'us.anthropic.claude-sonnet-4-20250514-v1:0',
      prism: 'core:bedrock'
    },
    { 
      id: 'bedrock-claude-4-opus', 
      label: 'Bedrock - Claude Opus 4.0', 
      backend: 'bedrock', 
      model: 'us.anthropic.claude-opus-4-20250514-v1:0',
      prism: 'core:bedrock'
    },
    { 
      id: 'bedrock-claude-3-7-sonnet', 
      label: 'Bedrock - Claude Sonnet 3.7', 
      backend: 'bedrock', 
      model: 'us.anthropic.claude-3-7-sonnet-20250219-v1:0',
      prism: 'core:bedrock'
    },
    { 
      id: 'bedrock-claude-3-5-sonnet', 
      label: 'Bedrock - Claude Sonnet 3.5', 
      backend: 'bedrock', 
      model: 'us.anthropic.claude-3-5-sonnet-20241022-v2:0',
      prism: 'core:bedrock'
    },
    { 
      id: 'bedrock-deepseek-r1', 
      label: 'Bedrock - DeepSeek R1', 
      backend: 'bedrock', 
      model: 'us.deepseek.r1-v1:0',
      prism: 'core:bedrock'
    },
    { 
      id: 'bedrock-llama3', 
      label: 'Bedrock - Llama 3', 
      backend: 'bedrock', 
      model: 'us.meta.llama3-1-405b-instruct-v1:0',
      prism: 'core:bedrock'
    },
    { 
      id: 'bedrock-nova', 
      label: 'Bedrock - AWS Nova', 
      backend: 'bedrock', 
      model: 'us.amazon.nova-pro-v1:0',
      prism: 'core:bedrock'
    },
    // AWS Q Models
    { 
      id: 'q-claude-4-sonnet', 
      label: 'AWS Q - Claude Sonnet 4.0', 
      backend: 'q', 
      model: 'claude-4-sonnet',
      prism: 'core:q'
    },
    { 
      id: 'q-claude-3-7-sonnet', 
      label: 'AWS Q - Claude Sonnet 3.7', 
      backend: 'q', 
      model: 'claude-3.7-sonnet',
      prism: 'core:q'
    },
    { 
      id: 'q-claude-3-5-sonnet', 
      label: 'AWS Q - Claude Sonnet 3.5', 
      backend: 'q', 
      model: 'claude-3.5-sonnet',
      prism: 'core:q'
    }
  ];

  // Get current model option
  const getCurrentModelOption = () => {
    return MODEL_OPTIONS.find(option => option.id === selectedModelOption) || MODEL_OPTIONS[0];
  };
  
  // Initialize chat services
  useEffect(() => {
    if (connectionManager) {
      chatServiceRef.current = new ChatService(connectionManager);
      agentServiceRef.current = new AgentChatService(connectionManager);
      // Set default model based on current option
      const currentOption = getCurrentModelOption();
      chatServiceRef.current.setModel(currentOption.model);
      chatServiceRef.current.setPrism(currentOption.prism);
      agentServiceRef.current.setModel(currentOption.model);
      agentServiceRef.current.setPrism(currentOption.prism);
    }
  }, [connectionManager, selectedModelOption]);
  
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
      if (agentMode) {
        // Agent mode: handle structured events from agent prism
        setAgentProgress([]);
        
        const onEvent = (eventData) => {
          const processedEvent = agentServiceRef.current.processEvent(eventData);
          if (processedEvent) {
            switch (processedEvent.display) {
              case 'message':
                setMessages(currentMessages => [
                  ...currentMessages,
                  { role: 'assistant', content: processedEvent.content, type: 'ai_response' }
                ]);
                setIsTyping(false);
                break;
              case 'reasoning':
                setMessages(currentMessages => {
                  const newMessages = [...currentMessages];
                  const lastMessage = newMessages[newMessages.length - 1];
                  if (lastMessage && lastMessage.role === 'assistant') {
                    lastMessage.reasoning = processedEvent.content;
                  }
                  return newMessages;
                });
                break;
              case 'progress':
                setAgentProgress(current => [...current, processedEvent.content]);
                break;
              case 'action_result':
                setMessages(currentMessages => [
                  ...currentMessages,
                  { 
                    role: 'system', 
                    content: processedEvent, 
                    type: 'action_result'
                  }
                ]);
                break;
              case 'completion':
                setIsTyping(false);
                setAgentProgress([]); // Clear progress indicators
                break;
            }
          }
        };
        
        await agentServiceRef.current.sendMessage(userMessage, contextFiles, onEvent);
      } else {
        // Direct AI mode: handle token streaming from bedrock
        const assistantMessageIndex = updatedMessages.length;
        setMessages([...updatedMessages, { role: 'assistant', content: '' }]);
        
        const onToken = (data) => {
          if (data && data.token) {
            setMessages(currentMessages => {
              const newMessages = [...currentMessages];
              if (newMessages[assistantMessageIndex].content === '') {
                setIsTyping(false);
              }
              newMessages[assistantMessageIndex] = {
                ...newMessages[assistantMessageIndex],
                content: newMessages[assistantMessageIndex].content + data.token
              };
              return newMessages;
            });
          }
        };
        
        await chatServiceRef.current.sendMessage(
          updatedMessages,
          userMessage,
          contextFiles,
          onToken
        );
      }
      
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
    setExpandedActions({}); // Clear expanded actions state
  };

  // Toggle action expansion
  const toggleActionExpanded = (index) => {
    setExpandedActions(prev => ({
      ...prev,
      [index]: !prev[index]
    }));
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
  
  // Render action result card for agent mode
  const renderActionResult = (actionEvent, index) => {
    const { action, success, data, error } = actionEvent.content;
    const expanded = expandedActions[index] || false;
    
    return (
      <Box key={index} sx={{ mb: 2, display: 'flex', justifyContent: 'flex-start' }}>
        <Card 
          elevation={2}
          sx={{ 
            maxWidth: '80%',
            border: success ? '1px solid #4caf50' : '1px solid #f44336',
            borderRadius: 2
          }}
        >
          <Box sx={{ p: 2 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              {success ? (
                <CheckCircleIcon sx={{ color: 'success.main', mr: 1 }} />
              ) : (
                <ErrorIcon sx={{ color: 'error.main', mr: 1 }} />
              )}
              <Typography variant="subtitle2" sx={{ fontWeight: 'bold', flex: 1 }}>
                {action.prism} · {action.frequency}
              </Typography>
              
              {/* Toggle button for data/error visibility */}
              {((success && data) || (!success && error)) && (
                <IconButton
                  size="small"
                  onClick={() => toggleActionExpanded(index)}
                  sx={{ ml: 1 }}
                >
                  {expanded ? <ExpandLessIcon /> : <ExpandMoreIcon />}
                </IconButton>
              )}
            </Box>
            
            <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
              {action.description}
            </Typography>
            
            {/* Show summary when collapsed */}
            {!expanded && success && data && (
              <Typography variant="caption" color="text.secondary" sx={{ fontStyle: 'italic' }}>
                Result available - click to expand
              </Typography>
            )}
            
            {!expanded && !success && error && (
              <Typography variant="caption" color="error.main" sx={{ fontStyle: 'italic' }}>
                Error details available - click to expand
              </Typography>
            )}
            
            {/* Collapsible result data */}
            <Collapse in={expanded}>
              {success && data && (
                <Box sx={{ mt: 2 }}>
                  <Typography variant="caption" color="text.secondary">
                    Result:
                  </Typography>
                  <Paper 
                    sx={{ 
                      p: 1, 
                      mt: 0.5, 
                      bgcolor: 'rgba(76, 175, 80, 0.05)',
                      border: '1px solid rgba(76, 175, 80, 0.2)',
                      maxHeight: '300px',
                      overflowY: 'auto'
                    }}
                  >
                    <Typography variant="body2" component="pre" sx={{ 
                      whiteSpace: 'pre-wrap',
                      fontSize: '0.8rem',
                      fontFamily: 'monospace'
                    }}>
                      {JSON.stringify(data, null, 2)}
                    </Typography>
                  </Paper>
                </Box>
              )}
              
              {!success && error && (
                <Box sx={{ mt: 2 }}>
                  <Typography variant="caption" color="error.main">
                    Error:
                  </Typography>
                  <Paper 
                    sx={{ 
                      p: 1, 
                      mt: 0.5,
                      bgcolor: 'rgba(244, 67, 54, 0.05)',
                      border: '1px solid rgba(244, 67, 54, 0.2)'
                    }}
                  >
                    <Typography variant="body2" color="error.main">
                      {error}
                    </Typography>
                  </Paper>
                </Box>
              )}
            </Collapse>
          </Box>
        </Card>
      </Box>
    );
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
    } else if (message.role === 'system' && message.type === 'action_result') {
      // Render action result cards for agent mode
      return renderActionResult(message, index);
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
              bgcolor: message.type === 'ai_response' && agentMode ? 'rgba(25, 118, 210, 0.05)' : 'background.paper',
              color: 'text.primary',
              borderTopRightRadius: 2,
              borderTopLeftRadius: 0,
              borderLeft: message.type === 'ai_response' && agentMode ? '4px solid #1976d2' : 'none'
            }}
          >
            {message.type === 'ai_response' && agentMode && (
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                <SmartToyIcon fontSize="small" sx={{ color: 'primary.main', mr: 1 }} />
                <Typography variant="caption" sx={{ color: 'primary.main', fontWeight: 'bold' }}>
                  AI AGENT
                </Typography>
              </Box>
            )}
            
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
            
            {/* Show agent reasoning if available */}
            {message.reasoning && showReasoning && (
              <Box sx={{
                bgcolor: 'rgba(144, 202, 249, 0.1)',
                borderLeft: '4px solid',
                borderColor: 'info.main',
                pl: 1.5,
                py: 1,
                my: 1,
                borderRadius: '0 4px 4px 0',
                position: 'relative'
              }}>
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
                <ReactMarkdown components={MarkdownComponents}>
                  {message.reasoning}
                </ReactMarkdown>
              </Box>
            )}
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
          <FormControlLabel
            control={
              <Switch
                checked={agentMode}
                onChange={(e) => setAgentMode(e.target.checked)}
                color="primary"
              />
            }
            label={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                <SmartToyIcon fontSize="small" />
                <Typography variant="body2">Agent Mode</Typography>
              </Box>
            }
          />
          <FormControl variant="outlined" size="small" sx={{ minWidth: 200 }}>
            <InputLabel>Model</InputLabel>
            <Select
              value={selectedModelOption}
              onChange={(e) => {
                setSelectedModelOption(e.target.value);
                const selectedOption = MODEL_OPTIONS.find(option => option.id === e.target.value);
                if (selectedOption && chatServiceRef.current) {
                  chatServiceRef.current.setModel(selectedOption.model);
                  chatServiceRef.current.setPrism(selectedOption.prism);
                }
                if (selectedOption && agentServiceRef.current) {
                  agentServiceRef.current.setModel(selectedOption.model);
                  agentServiceRef.current.setPrism(selectedOption.prism);
                }
              }}
              label="Model"
            >
              {MODEL_OPTIONS.map((option) => (
                <MenuItem key={option.id} value={option.id}>
                  {option.label}
                </MenuItem>
              ))}
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
              {agentMode ? "AI Agent is working..." : "AI is typing..."}
            </Typography>
          </Box>
        )}
        
        {/* Agent progress indicators */}
        {agentMode && agentProgress.length > 0 && (
          <Box sx={{ mb: 2 }}>
            {agentProgress.map((progress, index) => (
              <Box key={index} sx={{ mb: 1, ml: 2 }}>
                <Card elevation={1} sx={{ p: 1.5, bgcolor: 'rgba(25, 118, 210, 0.05)' }}>
                  <Box sx={{ display: 'flex', alignItems: 'center' }}>
                    <PlayArrowIcon fontSize="small" sx={{ color: 'primary.main', mr: 1 }} />
                    <Typography variant="body2" color="primary.main">
                      {progress}
                    </Typography>
                  </Box>
                  <LinearProgress 
                    sx={{ 
                      mt: 1, 
                      height: 2,
                      bgcolor: 'rgba(25, 118, 210, 0.1)',
                      '& .MuiLinearProgress-bar': {
                        bgcolor: 'primary.main'
                      }
                    }} 
                  />
                </Card>
              </Box>
            ))}
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
