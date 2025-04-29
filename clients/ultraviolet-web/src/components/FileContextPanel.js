import React from 'react';
import { 
  Box, 
  Button, 
  Drawer, 
  IconButton, 
  List, 
  ListItem, 
  ListItemText, 
  Typography 
} from '@mui/material';
import AttachFileIcon from '@mui/icons-material/AttachFile';
import DeleteIcon from '@mui/icons-material/Delete';

/**
 * FileContextPanel component for managing file context
 * @param {Object} props - Component props
 * @param {boolean} props.open - Whether the panel is open
 * @param {function} props.onClose - Function to call when closing the panel
 * @param {Array} props.files - Array of file objects
 * @param {function} props.onAddFiles - Function to call when adding files
 * @param {function} props.onRemoveFile - Function to call when removing a file
 */
function FileContextPanel({ open, onClose, files, onAddFiles, onRemoveFile }) {
  // Handle file selection
  const handleFileSelect = (event) => {
    const selectedFiles = Array.from(event.target.files);
    if (!selectedFiles.length) return;
    
    const newFiles = [];
    let filesProcessed = 0;
    
    selectedFiles.forEach(selectedFile => {
      const reader = new FileReader();
      reader.onload = (e) => {
        const fileContent = e.target.result;
        newFiles.push({
          id: generateUUID(),
          name: selectedFile.name,
          content: fileContent,
          size: selectedFile.size
        });
        
        filesProcessed++;
        if (filesProcessed === selectedFiles.length) {
          // All files processed, now call onAddFiles once with all files
          onAddFiles(newFiles);
        }
      };
      reader.readAsText(selectedFile);
    });
  };
  
  // Generate a UUID for file IDs
  const generateUUID = () => {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
      const r = Math.random() * 16 | 0;
      const v = c === 'x' ? r : (r & 0x3 | 0x8);
      return v.toString(16);
    });
  };
  
  // Format file size
  const formatFileSize = (bytes) => {
    if (bytes < 1024) return bytes + ' B';
    else if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
    else return (bytes / 1048576).toFixed(1) + ' MB';
  };
  
  // Calculate total size of all files
  const totalSize = files.reduce((total, file) => total + file.size, 0);
  
  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
    >
      <Box sx={{ width: 300, p: 2 }}>
        <Typography variant="h6" gutterBottom>Context Files</Typography>
        
        {files.length === 0 ? (
          <Typography variant="body2" color="text.secondary" sx={{ my: 2 }}>
            No files added to context yet. Add files to provide additional context to the AI.
          </Typography>
        ) : (
          <>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {files.length} file{files.length !== 1 ? 's' : ''} ({formatFileSize(totalSize)})
            </Typography>
            
            <List sx={{ maxHeight: '50vh', overflow: 'auto' }}>
              {files.map(file => (
                <ListItem
                  key={file.id}
                  secondaryAction={
                    <IconButton 
                      edge="end" 
                      aria-label="delete" 
                      onClick={() => onRemoveFile(file.id)}
                      size="small"
                    >
                      <DeleteIcon fontSize="small" />
                    </IconButton>
                  }
                  sx={{ 
                    bgcolor: 'background.paper', 
                    mb: 1, 
                    borderRadius: 1,
                    border: '1px solid',
                    borderColor: 'divider'
                  }}
                >
                  <ListItemText
                    primary={file.name}
                    secondary={formatFileSize(file.size)}
                    primaryTypographyProps={{ 
                      variant: 'body2',
                      sx: { 
                        whiteSpace: 'nowrap',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis'
                      }
                    }}
                    secondaryTypographyProps={{ 
                      variant: 'caption'
                    }}
                  />
                </ListItem>
              ))}
            </List>
          </>
        )}
        
        <Box sx={{ mt: 2 }}>
          <Button
            variant="contained"
            component="label"
            startIcon={<AttachFileIcon />}
            fullWidth
          >
            Add Files
            <input
              type="file"
              hidden
              multiple
              onChange={handleFileSelect}
            />
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}

export default FileContextPanel;
