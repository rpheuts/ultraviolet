import React from 'react';
import { 
  Box, 
  CircularProgress, 
  Divider, 
  List, 
  ListItem, 
  ListItemButton, 
  ListItemText, 
  Paper, 
  Typography 
} from '@mui/material';

/**
 * Navigation component displays a list of prisms grouped by namespace
 * @param {Object} props - Component props
 * @param {Array} props.prisms - Array of prism info objects
 * @param {boolean} props.loading - Whether prisms are being loaded
 * @param {string} props.selectedPrism - Currently selected prism ID
 * @param {function} props.onSelectPrism - Function to call when a prism is selected
 * @param {boolean} props.connected - Whether the connection is established
 */
function Navigation({ prisms, loading, selectedPrism, onSelectPrism, connected = true }) {
  // Group prisms by namespace
  const groupedPrisms = prisms.reduce((groups, prism) => {
    const { namespace } = prism;
    if (!groups[namespace]) {
      groups[namespace] = [];
    }
    groups[namespace].push(prism);
    return groups;
  }, {});
  
  return (
    <Paper 
      sx={{ 
        width: 250, 
        height: '100%',
        overflow: 'auto',
        borderRadius: 0,
        borderRight: '1px solid',
        borderColor: 'divider',
      }}
      elevation={0}
    >
      <Box sx={{ p: 2 }}>
        <Typography variant="h6" gutterBottom>Prisms</Typography>
      </Box>
      
      {!connected ? (
        <Box 
          sx={{ 
            p: 2, 
            textAlign: 'center',
            border: '1px dashed',
            borderColor: 'error.main',
            borderRadius: 1,
            mx: 2,
            color: 'error.main'
          }}
        >
          <Typography variant="body2">Not connected to server</Typography>
          <Typography variant="body2">Please check your connection</Typography>
        </Box>
      ) : loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', p: 3 }}>
          <CircularProgress size={24} />
          <Typography variant="body2" sx={{ ml: 1 }}>
            Loading prisms...
          </Typography>
        </Box>
      ) : (
        <List sx={{ px: 1 }} dense>
          {Object.entries(groupedPrisms).map(([namespace, prismList], index, array) => (
            <React.Fragment key={namespace}>
              <ListItem sx={{ py: 0 }}>
                <Typography 
                  variant="subtitle2" 
                  color="text.secondary"
                  sx={{ 
                    fontSize: '0.75rem', 
                    textTransform: 'uppercase',
                    letterSpacing: '0.08em',
                    fontWeight: 'bold',
                    py: 1
                  }}
                >
                  {namespace}
                </Typography>
              </ListItem>
              
              <List disablePadding>
                {prismList.map(prism => {
                  const prismId = `${prism.namespace}:${prism.name}`;
                  const isSelected = selectedPrism === prismId;
                  
                  return (
                    <ListItem key={prismId} disablePadding>
                      <ListItemButton 
                        selected={isSelected}
                        onClick={() => onSelectPrism(prismId)}
                        sx={{ 
                          borderRadius: 1,
                          py: 0.5,
                          '&.Mui-selected': {
                            backgroundColor: 'primary.dark',
                            '&:hover': {
                              backgroundColor: 'primary.dark',
                            }
                          }
                        }}
                      >
                        <ListItemText 
                          primary={prism.name} 
                          primaryTypographyProps={{ 
                            variant: 'body2',
                            sx: { 
                              fontWeight: isSelected ? 'bold' : 'normal',
                            }
                          }}
                        />
                      </ListItemButton>
                    </ListItem>
                  );
                })}
              </List>
              
              {index < array.length - 1 && (
                <Divider sx={{ my: 1 }} />
              )}
            </React.Fragment>
          ))}
          
          {Object.keys(groupedPrisms).length === 0 && (
            <ListItem>
              <Typography variant="body2" color="text.secondary" sx={{ fontStyle: 'italic' }}>
                No prisms available
              </Typography>
            </ListItem>
          )}
        </List>
      )}
    </Paper>
  );
}

export default Navigation;
