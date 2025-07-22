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
import HistoryIcon from '@mui/icons-material/History';
import CompressIcon from '@mui/icons-material/Compress';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import FileContextPanel from './FileContextPanel';
import ConversationHistory from './ConversationHistory';
import ChatService from '../services/ChatService';
import AgentChatService from '../services/AgentChatService';
import ConversationStorage from '../services/ConversationStorage';

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
  const [lastFailedPrompt, setLastFailedPrompt] = useState(null);
  const [lastFailedContextFiles, setLastFailedContextFiles] = useState([]);
  const [contextFiles, setContextFiles] = useState([]);
  const [filesPanelOpen, setFilesPanelOpen] = useState(false);
  const [selectedModelOption, setSelectedModelOption] = useState("q-claude-4-sonnet");
  const [showReasoning, setShowReasoning] = useState(false);
  const [agentMode, setAgentMode] = useState(true);
  const [agentProgress, setAgentProgress] = useState([]);
  const [expandedActions, setExpandedActions] = useState({});
  const [currentSession, setCurrentSession] = useState(null);
  const [historyDialogOpen, setHistoryDialogOpen] = useState(false);
  const [tokenUsage, setTokenUsage] = useState(null);
  const [deleteConfirmId, setDeleteConfirmId] = useState(null);
  const messagesEndRef = useRef(null);
  const chatServiceRef = useRef(null);
  const agentServiceRef = useRef(null);
  const conversationStorageRef = useRef(null);

  // Model options with backend routing
  const MODEL_OPTIONS = [
    // Bedrock Models
    { 
      id: 'bedrock-claude-4-sonnet', 
      label: 'Bedrock - Claude Sonnet 4.0', 
      backend: 'bedrock', 
      model: 'us.anthropic.claude-sonnet-4-20250514-v1:0',
      prism: 'core:bedrock',
      contextLimit: 200000
    },
    { 
      id: 'bedrock-claude-4-opus', 
      label: 'Bedrock - Claude Opus 4.0', 
      backend: 'bedrock', 
      model: 'us.anthropic.claude-opus-4-20250514-v1:0',
      prism: 'core:bedrock',
      contextLimit: 200000
    },
    { 
      id: 'bedrock-claude-3-7-sonnet', 
      label: 'Bedrock - Claude Sonnet 3.7', 
      backend: 'bedrock', 
      model: 'us.anthropic.claude-3-7-sonnet-20250219-v1:0',
      prism: 'core:bedrock',
      contextLimit: 200000
    },
    { 
      id: 'bedrock-claude-3-5-sonnet', 
      label: 'Bedrock - Claude Sonnet 3.5', 
      backend: 'bedrock', 
      model: 'us.anthropic.claude-3-5-sonnet-20241022-v2:0',
      prism: 'core:bedrock',
      contextLimit: 200000
    },
    { 
      id: 'bedrock-deepseek-r1', 
      label: 'Bedrock - DeepSeek R1', 
      backend: 'bedrock', 
      model: 'us.deepseek.r1-v1:0',
      prism: 'core:bedrock',
      contextLimit: 200000
    },
    { 
      id: 'bedrock-llama3', 
      label: 'Bedrock - Llama 3', 
      backend: 'bedrock', 
      model: 'us.meta.llama3-1-405b-instruct-v1:0',
      prism: 'core:bedrock',
      contextLimit: 200000
    },
    { 
      id: 'bedrock-nova', 
      label: 'Bedrock - AWS Nova', 
      backend: 'bedrock', 
      model: 'us.amazon.nova-pro-v1:0',
      prism: 'core:bedrock',
      contextLimit: 200000
    },
    // AWS Q Models
    { 
      id: 'q-claude-4-sonnet', 
      label: 'AWS Q - Claude Sonnet 4.0', 
      backend: 'q', 
      model: 'claude-4-sonnet',
      prism: 'core:q',
      contextLimit: 200000
    },
    { 
      id: 'q-claude-3-5-sonnet', 
      label: 'AWS Q - Claude Sonnet 3.5', 
      backend: 'q', 
      model: 'claude-3.5-sonnet',
      prism: 'core:q',
      contextLimit: 200000
    },
    { 
      id: 'ollama-gemma3-12b', 
      label: 'Ollama - Gemma 3 (12b)', 
      backend: 'ollama', 
      model: 'gemma3:12b',
      prism: 'core:ollama',
      contextLimit: 200000
    }
  ];

  // Get current model option
  const getCurrentModelOption = () => {
    return MODEL_OPTIONS.find(option => option.id === selectedModelOption) || MODEL_OPTIONS[0];
  };
  
  // Initialize conversation storage and chat services
  useEffect(() => {
    // Initialize conversation storage
    conversationStorageRef.current = new ConversationStorage();
    
    // Create or load session for this tab
    initializeSession();
    
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
  }, [connectionManager]);

  // Handle model changes
  useEffect(() => {
    const currentOption = getCurrentModelOption();
    if (chatServiceRef.current) {
      chatServiceRef.current.setModel(currentOption.model);
      chatServiceRef.current.setPrism(currentOption.prism);
    }
    if (agentServiceRef.current) {
      agentServiceRef.current.setModel(currentOption.model);
      agentServiceRef.current.setPrism(currentOption.prism);
    }
    
    // Update session model if we have a current session
    if (currentSession && conversationStorageRef.current) {
      conversationStorageRef.current.updateSessionMetadata(currentSession.id, {
        model: currentOption.model
      });
    }
  }, [selectedModelOption, currentSession]);

  // Handle agent mode changes
  useEffect(() => {
    // Update session agent mode if we have a current session
    if (currentSession && conversationStorageRef.current) {
      conversationStorageRef.current.updateSessionMetadata(currentSession.id, {
        agentMode: agentMode
      });
    }
  }, [agentMode, currentSession]);

  // Initialize or load conversation session
  const initializeSession = () => {
    if (!conversationStorageRef.current) return;

    // Check if there's a session ID in the URL hash (for future use)
    const urlHash = window.location.hash.substring(1);
    let session = null;

    if (urlHash.startsWith('session_')) {
      // Try to load existing session from URL
      session = conversationStorageRef.current.getSession(urlHash);
      if (session) {
        console.log('Loaded existing session from URL:', urlHash);
      }
    }

    if (!session) {
      // Create new session
      const currentOption = getCurrentModelOption();
      session = conversationStorageRef.current.createNewSession(currentOption.model, agentMode);
      console.log('Created new session:', session.id);
      
      // Update URL hash to include session ID (optional)
      window.location.hash = session.id;
    } else {
      // Restore model and agent mode from loaded session
      console.log('Restoring session settings:', session);
      
      // Find the model option that matches the session's model
      const sessionModelOption = MODEL_OPTIONS.find(option => option.model === session.model);
      if (sessionModelOption) {
        setSelectedModelOption(sessionModelOption.id);
      }
      
      // Restore agent mode if available in session metadata
      if (typeof session.agentMode === 'boolean') {
        setAgentMode(session.agentMode);
      }
    }

    setCurrentSession(session);
    
    // Load existing messages
    const existingMessages = conversationStorageRef.current.getMessages(session.id);
    setMessages(existingMessages);
    
    // Update browser tab title
    updateTabTitle(session.title);
  };

  // Update browser tab title
  const updateTabTitle = (conversationTitle) => {
    const baseTitle = 'UV Chat';
    if (conversationTitle && conversationTitle !== 'New Conversation') {
      document.title = `${baseTitle} - ${conversationTitle}`;
    } else {
      document.title = baseTitle;
    }
  };

  // Save message to storage
  const saveMessageToStorage = (message) => {
    if (currentSession && conversationStorageRef.current) {
      const savedMessage = conversationStorageRef.current.saveMessage(currentSession.id, message);
      
      // Update tab title if this was the first message
      if (messages.length === 0 && message.role === 'user') {
        const updatedSession = conversationStorageRef.current.getSession(currentSession.id);
        if (updatedSession) {
          setCurrentSession(updatedSession);
          updateTabTitle(updatedSession.title);
        }
      }
      
      return savedMessage;
    }
    return message;
  };
  
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
    
    // Create and save user message
    const userMessageObj = { role: 'user', content: userMessage };
    const savedUserMessage = saveMessageToStorage(userMessageObj);
    
    // Add user message to chat
    const updatedMessages = [
      ...messages,
      savedUserMessage
    ];
    setMessages(updatedMessages);
    
    // Start AI response
    setIsTyping(true);
    setError(null);
    setLastFailedPrompt(null);
    setLastFailedContextFiles([]);
    
    try {
      if (agentMode) {
        // Agent mode: handle structured events from agent prism
        setAgentProgress([]);
        
        const onEvent = (eventData) => {
          const processedEvent = agentServiceRef.current.processEvent(eventData);
          if (processedEvent) {
            switch (processedEvent.display) {
              case 'message':
                const assistantMessage = { role: 'assistant', content: processedEvent.content, type: 'ai_response' };
                const savedAssistantMessage = saveMessageToStorage(assistantMessage);
                setMessages(currentMessages => [
                  ...currentMessages,
                  savedAssistantMessage
                ]);
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
                // Clear progress notifications when action completes
                setAgentProgress(current => {
                  // Remove progress notifications related to this action
                  const actionDescription = processedEvent.action.description;
                  return current.filter(progress => 
                    !progress.includes(actionDescription) && 
                    !progress.includes('Executing:') &&
                    !progress.includes('✓') &&
                    !progress.includes('✗')
                  );
                });
                
                // Create and save action result message
                const actionResultMessage = { 
                  role: 'system', 
                  content: processedEvent, 
                  type: 'action_result'
                };
                const savedActionResult = saveMessageToStorage(actionResultMessage);
                
                setMessages(currentMessages => [
                  ...currentMessages,
                  savedActionResult
                ]);
                break;
              case 'usage':
                // Handle usage data from agent mode
                setTokenUsage(processedEvent.usage);
                break;
              case 'completion':
                setIsTyping(false);
                setAgentProgress([]); // Clear progress indicators
                break;
            }
          }
        };
        
        await agentServiceRef.current.sendMessage(updatedMessages, userMessage, contextFiles, onEvent);
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

        const onUsage = (usage) => {
          if (usage) {
            setTokenUsage(usage);
          }
        };
        
        await chatServiceRef.current.sendMessage(
          updatedMessages,
          userMessage,
          contextFiles,
          onToken,
          onUsage
        );
        
        // Save the completed assistant message to storage
        setMessages(currentMessages => {
          const lastMessage = currentMessages[currentMessages.length - 1];
          if (lastMessage && lastMessage.role === 'assistant') {
            const savedMessage = saveMessageToStorage(lastMessage);
            const newMessages = [...currentMessages];
            newMessages[newMessages.length - 1] = savedMessage;
            return newMessages;
          }
          return currentMessages;
        });
      }
      
      setIsTyping(false);
    } catch (err) {
      // Store failed prompt and context for retry functionality
      setLastFailedPrompt(userMessage);
      setLastFailedContextFiles([...contextFiles]);
      setError(`Error: ${err.message}`);
      setIsTyping(false);
    }
  };
  
  // Clear conversation
  const handleClearConversation = () => {
    setMessages([]);
    setError(null);
    setLastFailedPrompt(null);
    setLastFailedContextFiles([]);
    setExpandedActions({}); // Clear expanded actions state
  };

  // Retry the last failed prompt
  const handleRetry = async () => {
    if (!lastFailedPrompt) return;
    
    // Clear error state
    setError(null);
    
    // Retry with the exact same prompt and context files
    const userMessage = lastFailedPrompt;
    const retryContextFiles = lastFailedContextFiles;
    
    // Don't add the user message again - it's already in the conversation
    const updatedMessages = [...messages];
    
    // Start AI response
    setIsTyping(true);
    
    try {
      if (agentMode) {
        // Agent mode retry
        setAgentProgress([]);
        
        const onEvent = (eventData) => {
          const processedEvent = agentServiceRef.current.processEvent(eventData);
          if (processedEvent) {
            switch (processedEvent.display) {
              case 'message':
                const assistantMessage = { role: 'assistant', content: processedEvent.content, type: 'ai_response' };
                const savedAssistantMessage = saveMessageToStorage(assistantMessage);
                setMessages(currentMessages => [
                  ...currentMessages,
                  savedAssistantMessage
                ]);
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
                setAgentProgress(current => {
                  const actionDescription = processedEvent.action.description;
                  return current.filter(progress => 
                    !progress.includes(actionDescription) && 
                    !progress.includes('Executing:') &&
                    !progress.includes('✓') &&
                    !progress.includes('✗')
                  );
                });
                
                const actionResultMessage = { 
                  role: 'system', 
                  content: processedEvent, 
                  type: 'action_result'
                };
                const savedActionResult = saveMessageToStorage(actionResultMessage);
                
                setMessages(currentMessages => [
                  ...currentMessages,
                  savedActionResult
                ]);
                break;
              case 'usage':
                setTokenUsage(processedEvent.usage);
                break;
              case 'completion':
                setIsTyping(false);
                setAgentProgress([]);
                break;
            }
          }
        };
        
        await agentServiceRef.current.sendMessage(updatedMessages, userMessage, retryContextFiles, onEvent);
      } else {
        // Direct AI mode retry
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

        const onUsage = (usage) => {
          if (usage) {
            setTokenUsage(usage);
          }
        };
        
        await chatServiceRef.current.sendMessage(
          updatedMessages,
          userMessage,
          retryContextFiles,
          onToken,
          onUsage
        );
        
        setMessages(currentMessages => {
          const lastMessage = currentMessages[currentMessages.length - 1];
          if (lastMessage && lastMessage.role === 'assistant') {
            const savedMessage = saveMessageToStorage(lastMessage);
            const newMessages = [...currentMessages];
            newMessages[newMessages.length - 1] = savedMessage;
            return newMessages;
          }
          return currentMessages;
        });
      }
      
      // Clear retry state on success
      setLastFailedPrompt(null);
      setLastFailedContextFiles([]);
      setIsTyping(false);
    } catch (err) {
      // Keep the same failed prompt for another retry attempt
      setError(`Error: ${err.message}`);
      setIsTyping(false);
    }
  };

  // Edit and retry the last failed prompt
  const handleEditAndRetry = () => {
    if (!lastFailedPrompt) return;
    
    // Pre-fill the input with the failed prompt
    setInputValue(lastFailedPrompt);
    
    // Restore the context files that were used
    setContextFiles([...lastFailedContextFiles]);
    
    // Clear error state
    setError(null);
    setLastFailedPrompt(null);
    setLastFailedContextFiles([]);
  };

  // Clear error and continue normally
  const handleContinue = () => {
    setError(null);
    setLastFailedPrompt(null);
    setLastFailedContextFiles([]);
  };

  // Handle message deletion with two-click confirmation
  const handleDeleteMessage = (messageId, messageIndex) => {
    if (deleteConfirmId === messageId) {
      // Second click - actually delete the message
      const updatedMessages = messages.filter((_, index) => index !== messageIndex);
      setMessages(updatedMessages);
      
      // Update storage
      if (currentSession && conversationStorageRef.current && messageId) {
        conversationStorageRef.current.deleteMessage(currentSession.id, messageId);
        
        // Update session in state if title changed
        const updatedSession = conversationStorageRef.current.getSession(currentSession.id);
        if (updatedSession) {
          setCurrentSession(updatedSession);
          updateTabTitle(updatedSession.title);
        }
      }
      
      // Clear token usage since we can't recalculate it on frontend
      setTokenUsage(null);
      
      // Clear confirmation state
      setDeleteConfirmId(null);
      
      console.log('Deleted message:', messageId);
    } else {
      // First click - show confirmation
      setDeleteConfirmId(messageId);
      // Auto-cancel confirmation after 3 seconds
      setTimeout(() => {
        setDeleteConfirmId(null);
      }, 3000);
    }
  };

  // Handle context reduction by summarizing conversation
  const handleReduceContext = async () => {
    if (messages.length === 0 || isTyping) return;
    
    setIsTyping(true);
    setError(null);
    
    try {
      // Create summarization prompt
      const summaryPrompt = `The user requested to compress the context / prompt. Please provide a comprehensive but concise summary of our conversation that preserves all important context, decisions, information, and user preferences needed to continue our discussion effectively. Focus on:

1. Key topics and decisions made
2. Important information shared
3. User preferences or requirements established
4. Context needed for future responses
5. Any ongoing tasks or objectives

Please format this as a clear, well-organized summary that maintains conversation continuity.`;

      let summaryContent = '';
      
      if (agentMode) {
        // Agent mode summarization
        setAgentProgress(['Summarizing conversation...']);
        
        const onEvent = (eventData) => {
          const processedEvent = agentServiceRef.current.processEvent(eventData);
          if (processedEvent) {
            switch (processedEvent.display) {
              case 'message':
                summaryContent += processedEvent.content;
                break;
              case 'usage':
                setTokenUsage(processedEvent.usage);
                break;
              case 'completion':
                setIsTyping(false);
                setAgentProgress([]);
                break;
            }
          }
        };
        
        await agentServiceRef.current.sendMessage(messages, summaryPrompt, contextFiles, onEvent);
      } else {
        // Direct AI mode summarization
        const onToken = (data) => {
          if (data && data.token) {
            summaryContent += data.token;
          }
        };

        const onUsage = (usage) => {
          if (usage) {
            setTokenUsage(usage);
          }
        };
        
        await chatServiceRef.current.sendMessage(
          messages,
          summaryPrompt,
          contextFiles,
          onToken,
          onUsage
        );
      }
      
      // Create summary message
      const summaryMessage = {
        role: 'system',
        content: `**📋 Conversation Summary**\n\n${summaryContent}`,
        type: 'summary',
        timestamp: new Date().toISOString()
      };
      
      // Save summary message to storage
      const savedSummaryMessage = saveMessageToStorage(summaryMessage);
      
      // Keep the last user message and replace everything else with summary
      const lastUserMessageIndex = messages.length - 1;
      let messagesToKeep = [];
      
      // Find the last user message to preserve continuity
      for (let i = messages.length - 1; i >= 0; i--) {
        if (messages[i].role === 'user') {
          messagesToKeep = messages.slice(i); // Keep from last user message onwards
          break;
        }
      }
      
      // If no user message found, just use the summary
      if (messagesToKeep.length === 0) {
        messagesToKeep = [];
      }
      
      // Replace conversation with summary + recent messages
      const newMessages = [savedSummaryMessage, ...messagesToKeep];
      setMessages(newMessages);
      
      // Update storage with reduced conversation
      if (currentSession && conversationStorageRef.current) {
        // Clear existing messages and save new reduced set
        conversationStorageRef.current.clearMessages(currentSession.id);
        newMessages.forEach(msg => {
          conversationStorageRef.current.saveMessage(currentSession.id, msg);
        });
        
        // Update session metadata
        const updatedSession = conversationStorageRef.current.getSession(currentSession.id);
        if (updatedSession) {
          setCurrentSession(updatedSession);
          updateTabTitle(updatedSession.title);
        }
      }
      
      console.log('Context reduced successfully');
      setIsTyping(false);
      
    } catch (err) {
      setError(`Failed to reduce context: ${err.message}`);
      setIsTyping(false);
      setAgentProgress([]);
    }
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
    const messageId = message.id;
    
    if (isUser) {
      // User messages with delete button
      return (
        <Box
          key={index}
          sx={{
            display: 'flex',
            justifyContent: 'flex-end',
            mb: 2,
            position: 'relative',
            '&:hover .delete-button': {
              opacity: 1
            }
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
              borderTopLeftRadius: 2,
              position: 'relative'
            }}
          >
            <Typography variant="body1" sx={{ whiteSpace: 'pre-wrap' }}>
              {message.content}
            </Typography>
            
            {/* Delete button */}
            <Tooltip title={deleteConfirmId === messageId ? "Click again to confirm" : "Delete message"}>
              <IconButton
                className="delete-button"
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteMessage(messageId, index);
                }}
                sx={{
                  position: 'absolute',
                  top: 4,
                  right: 4,
                  opacity: 0,
                  transition: 'opacity 0.2s',
                  color: deleteConfirmId === messageId ? 'error.main' : 'rgba(255, 255, 255, 0.7)',
                  bgcolor: 'rgba(0, 0, 0, 0.2)',
                  '&:hover': {
                    bgcolor: 'rgba(0, 0, 0, 0.4)',
                    color: deleteConfirmId === messageId ? 'error.main' : 'white'
                  }
                }}
              >
                <DeleteIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Paper>
        </Box>
      );
    } else if (message.role === 'system' && message.type === 'action_result') {
      // Render action result cards for agent mode with delete button
      return (
        <Box key={index} sx={{ mb: 2, display: 'flex', justifyContent: 'flex-start', position: 'relative', '&:hover .delete-button': { opacity: 1 } }}>
          <Card 
            elevation={2}
            sx={{ 
              maxWidth: '80%',
              border: message.content.success ? '1px solid #4caf50' : '1px solid #f44336',
              borderRadius: 2,
              position: 'relative'
            }}
          >
            <Box sx={{ p: 2 }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                {message.content.success ? (
                  <CheckCircleIcon sx={{ color: 'success.main', mr: 1 }} />
                ) : (
                  <ErrorIcon sx={{ color: 'error.main', mr: 1 }} />
                )}
                <Typography variant="subtitle2" sx={{ fontWeight: 'bold', flex: 1 }}>
                  {message.content.action.prism} · {message.content.action.frequency}
                </Typography>
                
                {/* Toggle button for data/error visibility */}
                {((message.content.success && message.content.data) || (!message.content.success && message.content.error)) && (
                  <IconButton
                    size="small"
                    onClick={() => toggleActionExpanded(index)}
                    sx={{ ml: 1 }}
                  >
                    {expandedActions[index] ? <ExpandLessIcon /> : <ExpandMoreIcon />}
                  </IconButton>
                )}
              </Box>
              
              <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                {message.content.action.description}
              </Typography>
              
              {/* Show summary when collapsed */}
              {!expandedActions[index] && message.content.success && message.content.data && (
                <Typography variant="caption" color="text.secondary" sx={{ fontStyle: 'italic' }}>
                  Result available - click to expand
                </Typography>
              )}
              
              {!expandedActions[index] && !message.content.success && message.content.error && (
                <Typography variant="caption" color="error.main" sx={{ fontStyle: 'italic' }}>
                  Error details available - click to expand
                </Typography>
              )}
              
              {/* Collapsible result data */}
              <Collapse in={expandedActions[index]}>
                {message.content.success && message.content.data && (
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
                        {JSON.stringify(message.content.data, null, 2)}
                      </Typography>
                    </Paper>
                  </Box>
                )}
                
                {!message.content.success && message.content.error && (
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
                        {message.content.error}
                      </Typography>
                    </Paper>
                  </Box>
                )}
              </Collapse>
            </Box>
            
            {/* Delete button for action result */}
            <Tooltip title={deleteConfirmId === messageId ? "Click again to confirm" : "Delete message"}>
              <IconButton
                className="delete-button"
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteMessage(messageId, index);
                }}
                sx={{
                  position: 'absolute',
                  top: 4,
                  right: 4,
                  opacity: 0,
                  transition: 'opacity 0.2s',
                  color: deleteConfirmId === messageId ? 'error.main' : 'text.secondary',
                  bgcolor: 'rgba(255, 255, 255, 0.8)',
                  '&:hover': {
                    bgcolor: 'rgba(255, 255, 255, 1)',
                    color: deleteConfirmId === messageId ? 'error.main' : 'text.primary'
                  }
                }}
              >
                <DeleteIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Card>
        </Box>
      );
    } else if (message.role === 'system' && message.type === 'summary') {
      // Render summary messages with special styling
      return (
        <Box
          key={index}
          sx={{
            display: 'flex',
            justifyContent: 'center',
            mb: 2,
            position: 'relative',
            '&:hover .delete-button': {
              opacity: 1
            }
          }}
        >
          <Paper
            elevation={2}
            sx={{
              p: 2,
              maxWidth: '90%',
              borderRadius: 2,
              bgcolor: 'rgba(255, 193, 7, 0.1)',
              color: 'text.primary',
              border: '2px solid rgba(255, 193, 7, 0.3)',
              position: 'relative'
            }}
          >
            <ReactMarkdown components={MarkdownComponents}>
              {message.content}
            </ReactMarkdown>
            
            {/* Delete button for summary messages */}
            <Tooltip title={deleteConfirmId === messageId ? "Click again to confirm" : "Delete summary"}>
              <IconButton
                className="delete-button"
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteMessage(messageId, index);
                }}
                sx={{
                  position: 'absolute',
                  top: 4,
                  right: 4,
                  opacity: 0,
                  transition: 'opacity 0.2s',
                  color: deleteConfirmId === messageId ? 'error.main' : 'text.secondary',
                  bgcolor: 'rgba(255, 255, 255, 0.8)',
                  '&:hover': {
                    bgcolor: 'rgba(255, 255, 255, 1)',
                    color: deleteConfirmId === messageId ? 'error.main' : 'text.primary'
                  }
                }}
              >
                <DeleteIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Paper>
        </Box>
      );
    } else {
      // Process assistant messages to handle reasoning sections with delete button
      const processedContent = processMessageContent(message.content);
      
      return (
        <Box
          key={index}
          sx={{
            display: 'flex',
            justifyContent: 'flex-start',
            mb: 2,
            position: 'relative',
            '&:hover .delete-button': {
              opacity: 1
            }
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
              borderLeft: message.type === 'ai_response' && agentMode ? '4px solid #1976d2' : 'none',
              position: 'relative'
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
            
            {/* Delete button for assistant messages */}
            <Tooltip title={deleteConfirmId === messageId ? "Click again to confirm" : "Delete message"}>
              <IconButton
                className="delete-button"
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteMessage(messageId, index);
                }}
                sx={{
                  position: 'absolute',
                  top: 4,
                  right: 4,
                  opacity: 0,
                  transition: 'opacity 0.2s',
                  color: deleteConfirmId === messageId ? 'error.main' : 'text.secondary',
                  bgcolor: 'rgba(255, 255, 255, 0.8)',
                  '&:hover': {
                    bgcolor: 'rgba(255, 255, 255, 1)',
                    color: deleteConfirmId === messageId ? 'error.main' : 'text.primary'
                  }
                }}
              >
                <DeleteIcon fontSize="small" />
              </IconButton>
            </Tooltip>
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
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
          {/* Token usage display */}
          {tokenUsage ? (
            <Tooltip title={`Context window usage: ${tokenUsage.prompt_tokens} / ${getCurrentModelOption().contextLimit} tokens`}>
              <Chip
                label={`${tokenUsage.prompt_tokens.toLocaleString()} tokens`}
                size="small"
                color={tokenUsage.prompt_tokens > getCurrentModelOption().contextLimit * 0.8 ? "warning" : "default"}
                variant="outlined"
                sx={{ mr: 1 }}
              />
            </Tooltip>
          ) : (
            messages.length > 0 && (
              <Tooltip title="Token usage will update after next AI response">
                <Chip
                  label="Token usage will refresh"
                  size="small"
                  color="default"
                  variant="outlined"
                  sx={{ mr: 1, fontStyle: 'italic' }}
                />
              </Tooltip>
            )
          )}
          
          <Tooltip title="Conversation History">
            <IconButton 
              color="inherit"
              onClick={() => setHistoryDialogOpen(true)}
            >
              <HistoryIcon />
            </IconButton>
          </Tooltip>
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
          <Tooltip title="Reduce context by summarizing conversation">
            <IconButton 
              color="inherit"
              onClick={handleReduceContext}
              disabled={messages.length === 0 || isTyping}
            >
              <CompressIcon />
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
      
      {/* Conversation History Dialog */}
      <ConversationHistory
        open={historyDialogOpen}
        onClose={() => setHistoryDialogOpen(false)}
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
        
        {/* Enhanced error message with retry options */}
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
            <Box sx={{ display: 'flex', alignItems: 'flex-start', mb: 2 }}>
              <ErrorIcon sx={{ mr: 1, mt: 0.25 }} />
              <Typography variant="body2" sx={{ flex: 1 }}>
                {error}
              </Typography>
            </Box>
            
            {/* Retry action buttons */}
            {lastFailedPrompt && (
              <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                <Button
                  variant="contained"
                  size="small"
                  onClick={handleRetry}
                  disabled={isTyping}
                  sx={{ 
                    bgcolor: 'rgba(255, 255, 255, 0.15)',
                    color: 'inherit',
                    '&:hover': {
                      bgcolor: 'rgba(255, 255, 255, 0.25)'
                    }
                  }}
                >
                  Retry
                </Button>
                <Button
                  variant="outlined"
                  size="small"
                  onClick={handleEditAndRetry}
                  disabled={isTyping}
                  sx={{ 
                    borderColor: 'rgba(255, 255, 255, 0.3)',
                    color: 'inherit',
                    '&:hover': {
                      borderColor: 'rgba(255, 255, 255, 0.5)',
                      bgcolor: 'rgba(255, 255, 255, 0.1)'
                    }
                  }}
                >
                  Edit & Retry
                </Button>
                <Button
                  variant="text"
                  size="small"
                  onClick={handleContinue}
                  sx={{ 
                    color: 'rgba(255, 255, 255, 0.7)',
                    '&:hover': {
                      color: 'inherit',
                      bgcolor: 'rgba(255, 255, 255, 0.1)'
                    }
                  }}
                >
                  Continue
                </Button>
              </Box>
            )}
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
