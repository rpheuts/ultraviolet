import React, { useState, useEffect, useRef } from 'react';
import { Box, Typography, Paper } from '@mui/material';

/**
 * StreamRenderer component for displaying streaming text responses
 * @param {Object} props - Component props
 * @param {Array} props.data - Array of response data objects
 * @param {Object} props.schema - JSON Schema for the response
 */
const StreamRenderer = ({ data, schema }) => {
  const [accumulatedText, setAccumulatedText] = useState('');
  const containerRef = useRef(null);
  
  // Extract the stream property name from the schema
  const streamProperty = schema && typeof schema['x-uv-stream'] === 'string' 
    ? schema['x-uv-stream'] 
    : null;
  
  // Accumulate text from the stream property in each data item
  useEffect(() => {
    if (!Array.isArray(data) || !streamProperty) {
      return;
    }
    
    let text = '';
    data.forEach(item => {
      if (item && typeof item === 'object' && streamProperty in item) {
        text += item[streamProperty];
      }
    });
    
    setAccumulatedText(text);
  }, [data, streamProperty]);
  
  // Auto-scroll to bottom when text updates
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [accumulatedText]);
  
  // If no data or invalid schema, show a message
  if (!Array.isArray(data) || data.length === 0 || !streamProperty) {
    return (
      <Box sx={{ textAlign: 'center', py: 3 }}>
        <Typography variant="body1" color="text.secondary">
          No streaming data available.
        </Typography>
      </Box>
    );
  }
  
  return (
    <Box
      ref={containerRef}
      sx={{
        whiteSpace: 'pre-wrap',
        fontFamily: 'monospace',
        fontSize: '0.9rem',
        lineHeight: 1.5,
        padding: 2,
        height: '400px',
        maxHeight: '400px',
        overflow: 'auto',
        backgroundColor: '#f5f5f5',
        borderRadius: '4px',
      }}
    >
      {accumulatedText || (
        <Typography variant="body2" color="text.secondary" sx={{ fontStyle: 'italic' }}>
          Waiting for data...
        </Typography>
      )}
    </Box>
  );
};

export default StreamRenderer;
