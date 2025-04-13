import React, { useState } from 'react';
import { 
  Box,
  Button,
  FormControl, 
  FormHelperText,
  IconButton,
  Paper,
  Typography
} from '@mui/material';
import { Add, Delete } from '@mui/icons-material';
import { validateArray } from '../utils/ValidationUtils';
import { getFieldLabel, getFieldDescription, getFieldType } from '../utils/SchemaParser';

// Import field components
import StringField from './StringField';
import NumberField from './NumberField';
import BooleanField from './BooleanField';
import ObjectField from './ObjectField';

/**
 * Array field component for form generation
 * @param {Object} props - Component props
 * @param {string} props.name - Field name
 * @param {Object} props.schema - JSON Schema for this field
 * @param {boolean} props.required - Whether the field is required
 * @param {Array} props.value - Current field value
 * @param {function} props.onChange - Change handler function
 * @param {string} props.error - Error message
 */
const ArrayField = ({ 
  name, 
  schema, 
  required = false, 
  value = [], 
  onChange, 
  error
}) => {
  const [touched, setTouched] = useState(false);
  
  // Get field label and description
  const label = getFieldLabel(name, schema);
  const description = getFieldDescription(schema);
  
  // Get item schema
  const itemSchema = schema?.items || {};
  const itemType = getFieldType(itemSchema);
  
  // Handle adding a new item
  const handleAddItem = () => {
    if (onChange) {
      // Create default value based on item type
      let defaultValue;
      switch (itemType) {
        case 'string':
          defaultValue = '';
          break;
        case 'number':
        case 'integer':
          defaultValue = 0;
          break;
        case 'boolean':
          defaultValue = false;
          break;
        case 'object':
          defaultValue = {};
          break;
        case 'array':
          defaultValue = [];
          break;
        default:
          defaultValue = '';
      }
      
      onChange([...(value || []), defaultValue]);
      setTouched(true);
    }
  };
  
  // Handle removing an item
  const handleRemoveItem = (index) => {
    if (onChange) {
      const newValue = [...(value || [])];
      newValue.splice(index, 1);
      onChange(newValue);
      setTouched(true);
    }
  };
  
  // Handle item change
  const handleItemChange = (index, itemValue) => {
    if (onChange) {
      const newValue = [...(value || [])];
      newValue[index] = itemValue;
      onChange(newValue);
    }
  };
  
  // Get validation error
  const validationError = touched ? (error || validateArray(value, { ...schema, required })) : null;
  
  // Render the appropriate field component based on item type
  const renderItemField = (itemValue, index) => {
    const itemProps = {
      name: `${name}[${index}]`,
      schema: itemSchema,
      value: itemValue,
      onChange: (newValue) => handleItemChange(index, newValue),
      required: false // Individual items are not required
    };
    
    switch (itemType) {
      case 'string':
        return <StringField {...itemProps} />;
      case 'number':
      case 'integer':
        return <NumberField {...itemProps} />;
      case 'boolean':
        return <BooleanField {...itemProps} />;
      case 'object':
        return <ObjectField {...itemProps} />;
      case 'array':
        return <ArrayField {...itemProps} />;
      default:
        return <StringField {...itemProps} />;
    }
  };
  
  return (
    <FormControl 
      fullWidth 
      margin="normal" 
      error={!!validationError}
    >
      <Typography variant="subtitle1" gutterBottom>
        {label}{required ? ' *' : ''}
      </Typography>
      
      {description && (
        <FormHelperText sx={{ marginTop: -1, marginBottom: 1 }}>
          {description}
        </FormHelperText>
      )}
      
      {validationError && (
        <FormHelperText error sx={{ marginTop: -1, marginBottom: 1 }}>
          {validationError}
        </FormHelperText>
      )}
      
      <Paper variant="outlined" sx={{ padding: 2, marginBottom: 2 }}>
        {Array.isArray(value) && value.length > 0 ? (
          value.map((item, index) => (
            <Box 
              key={index} 
              sx={{ 
                position: 'relative',
                marginBottom: 2,
                paddingRight: 5
              }}
            >
              {renderItemField(item, index)}
              <IconButton
                size="small"
                color="error"
                onClick={() => handleRemoveItem(index)}
                sx={{
                  position: 'absolute',
                  top: 0,
                  right: 0
                }}
              >
                <Delete />
              </IconButton>
            </Box>
          ))
        ) : (
          <Typography variant="body2" color="text.secondary" align="center">
            No items. Click "Add Item" to add one.
          </Typography>
        )}
        
        <Button
          startIcon={<Add />}
          onClick={handleAddItem}
          variant="outlined"
          size="small"
          fullWidth
          sx={{ marginTop: 1 }}
        >
          Add Item
        </Button>
      </Paper>
    </FormControl>
  );
};

export default ArrayField;
