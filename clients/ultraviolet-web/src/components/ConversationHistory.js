import React, { useState, useEffect } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  InputAdornment,
  List,
  ListItem,
  TextField,
  Tooltip,
  Typography
} from '@mui/material';
import {
  Delete as DeleteIcon,
  History as HistoryIcon,
  Launch as LaunchIcon,
  Search as SearchIcon,
  Close as CloseIcon
} from '@mui/icons-material';
import ConversationStorage from '../services/ConversationStorage';

/**
 * ConversationHistory component for managing past conversations
 * @param {Object} props - Component props
 * @param {boolean} props.open - Whether the dialog is open
 * @param {function} props.onClose - Function to call when dialog should close
 */
function ConversationHistory({ open, onClose }) {
  const [conversations, setConversations] = useState([]);
  const [filteredConversations, setFilteredConversations] = useState([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [deleteConfirmId, setDeleteConfirmId] = useState(null);
  const [storageService] = useState(() => new ConversationStorage());

  // Load conversations when dialog opens
  useEffect(() => {
    if (open) {
      loadConversations();
    }
  }, [open]);

  // Filter conversations based on search query
  useEffect(() => {
    if (!searchQuery.trim()) {
      setFilteredConversations(conversations);
    } else {
      const query = searchQuery.toLowerCase();
      const filtered = conversations.filter(conv =>
        conv.title.toLowerCase().includes(query)
      );
      setFilteredConversations(filtered);
    }
  }, [conversations, searchQuery]);

  // Listen for storage changes from other tabs
  useEffect(() => {
    const handleStorageChange = () => {
      if (open) {
        loadConversations();
      }
    };

    window.addEventListener('conversationStorageChange', handleStorageChange);
    return () => {
      window.removeEventListener('conversationStorageChange', handleStorageChange);
    };
  }, [open]);

  // Load conversations from storage
  const loadConversations = () => {
    const recentConversations = storageService.getRecentSessions(50); // Load up to 50 recent conversations
    setConversations(recentConversations);
  };

  // Open conversation in new tab
  const openConversationInNewTab = (sessionId) => {
    const url = `${window.location.origin}${window.location.pathname}#${sessionId}`;
    window.open(url, '_blank');
  };

  // Delete conversation with confirmation
  const handleDeleteConversation = (sessionId) => {
    if (deleteConfirmId === sessionId) {
      // Confirmed - actually delete
      storageService.deleteSession(sessionId);
      loadConversations(); // Refresh the list
      setDeleteConfirmId(null);
    } else {
      // First click - show confirmation
      setDeleteConfirmId(sessionId);
      // Auto-cancel confirmation after 3 seconds
      setTimeout(() => {
        setDeleteConfirmId(null);
      }, 3000);
    }
  };

  // Format timestamp for display
  const formatTimestamp = (timestamp) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now - date;
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
    
    if (diffDays === 0) {
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } else if (diffDays === 1) {
      return 'Yesterday';
    } else if (diffDays < 7) {
      return `${diffDays} days ago`;
    } else {
      return date.toLocaleDateString();
    }
  };

  // Get model display name
  const getModelDisplayName = (model) => {
    if (model.includes('claude-4-sonnet')) return 'Claude 4 Sonnet';
    if (model.includes('claude-3.5-sonnet')) return 'Claude 3.5 Sonnet';
    if (model.includes('claude-3-7-sonnet')) return 'Claude 3.7 Sonnet';
    if (model.includes('deepseek')) return 'DeepSeek R1';
    if (model.includes('nova')) return 'AWS Nova';
    if (model.includes('llama')) return 'Llama 3';
    return model;
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="md"
      fullWidth
      PaperProps={{
        sx: {
          height: '80vh',
          maxHeight: '600px'
        }
      }}
    >
      <DialogTitle sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Box sx={{ display: 'flex', alignItems: 'center' }}>
          <HistoryIcon sx={{ mr: 1 }} />
          <Typography variant="h6">Conversation History</Typography>
        </Box>
        <IconButton onClick={onClose} size="small">
          <CloseIcon />
        </IconButton>
      </DialogTitle>

      <DialogContent sx={{ p: 0 }}>
        {/* Search Bar */}
        <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider' }}>
          <TextField
            fullWidth
            size="small"
            placeholder="Search conversations..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <SearchIcon />
                </InputAdornment>
              )
            }}
          />
        </Box>

        {/* Conversations List */}
        <Box sx={{ flex: 1, overflow: 'auto' }}>
          {filteredConversations.length === 0 ? (
            <Box sx={{ p: 4, textAlign: 'center' }}>
              <Typography variant="body1" color="text.secondary">
                {searchQuery ? 'No conversations match your search.' : 'No conversations found.'}
              </Typography>
              {!searchQuery && (
                <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
                  Start a conversation to see it appear here.
                </Typography>
              )}
            </Box>
          ) : (
            <List sx={{ p: 1 }}>
              {filteredConversations.map((conversation) => (
                <ListItem key={conversation.id} sx={{ p: 1 }}>
                  <Card 
                    sx={{ 
                      width: '100%',
                      cursor: 'pointer',
                      transition: 'all 0.2s',
                      '&:hover': {
                        boxShadow: 2,
                        transform: 'translateY(-1px)'
                      }
                    }}
                    onClick={() => openConversationInNewTab(conversation.id)}
                  >
                    <CardContent sx={{ p: 2, '&:last-child': { pb: 2 } }}>
                      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', mb: 1 }}>
                        <Typography 
                          variant="subtitle1" 
                          sx={{ 
                            fontWeight: 'medium',
                            flex: 1,
                            mr: 2,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            display: '-webkit-box',
                            WebkitLineClamp: 2,
                            WebkitBoxOrient: 'vertical'
                          }}
                        >
                          {conversation.title}
                        </Typography>
                        
                        <Box sx={{ display: 'flex', gap: 0.5 }}>
                          <Tooltip title="Open in new tab">
                            <IconButton
                              size="small"
                              onClick={(e) => {
                                e.stopPropagation();
                                openConversationInNewTab(conversation.id);
                              }}
                              sx={{ color: 'primary.main' }}
                            >
                              <LaunchIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                          
                          <Tooltip title={deleteConfirmId === conversation.id ? "Click again to confirm" : "Delete conversation"}>
                            <IconButton
                              size="small"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleDeleteConversation(conversation.id);
                              }}
                              sx={{ 
                                color: deleteConfirmId === conversation.id ? 'error.main' : 'text.secondary',
                                '&:hover': { color: 'error.main' }
                              }}
                            >
                              <DeleteIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                        </Box>
                      </Box>

                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
                        <Chip
                          label={getModelDisplayName(conversation.model)}
                          size="small"
                          variant="outlined"
                          sx={{ fontSize: '0.7rem', height: '20px' }}
                        />
                        <Typography variant="caption" color="text.secondary">
                          {conversation.messageCount} message{conversation.messageCount !== 1 ? 's' : ''}
                        </Typography>
                      </Box>

                      <Typography variant="caption" color="text.secondary">
                        {formatTimestamp(conversation.lastUpdatedAt)}
                      </Typography>
                    </CardContent>
                  </Card>
                </ListItem>
              ))}
            </List>
          )}
        </Box>
      </DialogContent>

      <DialogActions sx={{ p: 2, borderTop: 1, borderColor: 'divider' }}>
        <Typography variant="body2" color="text.secondary" sx={{ flex: 1 }}>
          {filteredConversations.length} conversation{filteredConversations.length !== 1 ? 's' : ''}
          {searchQuery && ` (filtered from ${conversations.length})`}
        </Typography>
        <Button onClick={onClose}>Close</Button>
      </DialogActions>
    </Dialog>
  );
}

export default ConversationHistory;
