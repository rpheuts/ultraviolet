import React from 'react';
import { Box, Typography, Paper } from '@mui/material';
import StreamRenderer from './StreamRenderer';

/**
 * ResponseRenderer component for displaying prism responses
 * @param {Object} props - Component props
 * @param {Array|Object} props.data - Response data to render
 * @param {Object} props.schema - JSON Schema for the response
 */
const ResponseRenderer = ({ data, schema }) => {
  // Check if this is a streaming response
  const isStreaming = schema && schema['x-uv-stream'] && Array.isArray(data);
  
  // Use StreamRenderer for streaming responses
  if (isStreaming) {
    return <StreamRenderer data={data} schema={schema} />;
  }
  
  // If no data, show a message
  if (!data) {
    return (
      <Box sx={{ textAlign: 'center', py: 3 }}>
        <Typography variant="body1" color="text.secondary">
          No response data available.
        </Typography>
      </Box>
    );
  }
  
  // Determine the type of data
  const isArray = Array.isArray(data);
  const isObject = !isArray && typeof data === 'object' && data !== null;
  const isPrimitive = !isArray && !isObject;
  
  // Render array data as a list or table
  if (isArray) {
    // If array is empty, show a message
    if (data.length === 0) {
      return (
        <Box sx={{ textAlign: 'center', py: 3 }}>
          <Typography variant="body1" color="text.secondary">
            Empty array response.
          </Typography>
        </Box>
      );
    }
    
    // If array contains objects, render as a table
    if (typeof data[0] === 'object' && data[0] !== null) {
      const headers = Object.keys(data[0]);
      
      return (
        <Box sx={{ overflowX: 'auto' }}>
          <table style={{ 
            width: '100%', 
            borderCollapse: 'collapse',
            border: '1px solid rgba(255, 255, 255, 0.12)'
          }}>
            <thead>
              <tr>
                {headers.map(header => (
                  <th key={header} style={{ 
                    padding: '12px', 
                    textAlign: 'left',
                    borderBottom: '2px solid rgba(255, 255, 255, 0.12)',
                    backgroundColor: 'rgba(255, 255, 255, 0.05)'
                  }}>
                    {header}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data.map((item, rowIndex) => (
                <tr key={rowIndex} style={{
                  backgroundColor: rowIndex % 2 === 0 ? 'transparent' : 'rgba(255, 255, 255, 0.03)'
                }}>
                  {headers.map(header => (
                    <td key={`${rowIndex}-${header}`} style={{ 
                      padding: '8px', 
                      borderBottom: '1px solid rgba(255, 255, 255, 0.12)'
                    }}>
                      {renderCellValue(item[header])}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </Box>
      );
    }
    
    // If array contains primitives, render as a list
    return (
      <Box component="ul" sx={{ 
        listStyleType: 'none',
        padding: 0,
        margin: 0
      }}>
        {data.map((item, index) => (
          <Box 
            component="li" 
            key={index}
            sx={{
              padding: '8px',
              borderBottom: '1px solid rgba(255, 255, 255, 0.12)',
              '&:last-child': {
                borderBottom: 'none'
              }
            }}
          >
            {renderCellValue(item)}
          </Box>
        ))}
      </Box>
    );
  }
  
  // Render object data as a property list
  if (isObject) {
    return (
      <Box component="dl" sx={{ 
        margin: 0,
        display: 'grid',
        gridTemplateColumns: 'auto 1fr',
        gap: '8px 16px'
      }}>
        {Object.entries(data).map(([key, value]) => (
          <React.Fragment key={key}>
            <Box component="dt" sx={{ 
              fontWeight: 'bold',
              color: 'text.secondary'
            }}>
              {key}:
            </Box>
            <Box component="dd" sx={{ margin: 0 }}>
              {renderCellValue(value)}
            </Box>
          </React.Fragment>
        ))}
      </Box>
    );
  }
  
  // Render primitive data directly
  return (
    <Box sx={{ py: 2 }}>
      <Typography variant="body1">
        {String(data)}
      </Typography>
    </Box>
  );
};

/**
 * Helper function to render a cell value based on its type
 * @param {any} value - The value to render
 * @returns {React.ReactNode} Rendered value
 */
const renderCellValue = (value) => {
  // Handle null/undefined
  if (value === null || value === undefined) {
    return <Typography variant="body2" color="text.disabled">null</Typography>;
  }
  
  // Handle objects (including arrays)
  if (typeof value === 'object') {
    return (
      <pre style={{ 
        margin: 0, 
        fontSize: '0.8rem',
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-word'
      }}>
        {JSON.stringify(value, null, 2)}
      </pre>
    );
  }
  
  // Handle booleans
  if (typeof value === 'boolean') {
    return value ? 'True' : 'False';
  }
  
  // Handle dates
  if (typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/.test(value)) {
    try {
      return new Date(value).toLocaleString();
    } catch (e) {
      return value;
    }
  }
  
  // Handle URLs
  if (typeof value === 'string' && value.startsWith('http')) {
    return (
      <a 
        href={value} 
        target="_blank" 
        rel="noopener noreferrer"
        style={{ color: '#2196f3', textDecoration: 'none' }}
      >
        {value}
      </a>
    );
  }
  
  // Default rendering
  return String(value);
};

export default ResponseRenderer;
