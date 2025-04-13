import React, { useState } from 'react';
import { 
  TextField, 
  FormControl, 
  FormHelperText,
  InputAdornment,
  IconButton
} from '@mui/material';
import { Visibility, VisibilityOff } from '@mui/icons-material';
import { validateString } from '../utils/ValidationUtils';
import { getFieldLabel, getFieldDescription } from '../utils/SchemaParser';

/**
 * String field component for form generation
 * @param {Object} props - Component props
 * @param {string} props.name - Field name
 * @param {Object} props.schema - JSON Schema for this field
 * @param {boolean} props.required - Whether the field is required
 * @param {string} props.value - Current field value
 * @param {function} props.onChange - Change handler function
 * @param {function} props.onBlur - Blur handler function
 * @param {string} props.error - Error message
 */
const StringField = ({ 
  name, 
  schema, 
  required = false, 
  value = '', 
  onChange, 
  onBlur,
  error
}) => {
  const [touched, setTouched] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  
  // Determine field type based on schema format
  const getInputType = () => {
    if (!schema) return 'text';
    
    if (schema.format === 'password') {
      return showPassword ? 'text' : 'password';
    }
    
    switch (schema.format) {
      case 'email':
        return 'email';
      case 'uri':
        return 'url';
      case 'date':
        return 'date';
      case 'date-time':
        return 'datetime-local';
      default:
        return 'text';
    }
  };
  
  // Handle field change
  const handleChange = (e) => {
    if (onChange) {
      onChange(e.target.value);
    }
  };
  
  // Handle field blur
  const handleBlur = (e) => {
    setTouched(true);
    if (onBlur) {
      onBlur(e);
    }
  };
  
  // Toggle password visibility
  const handleTogglePasswordVisibility = () => {
    setShowPassword(!showPassword);
  };
  
  // Get validation error
  const validationError = touched ? (error || validateString(value, { ...schema, required })) : null;
  
  // Get field label and description
  const label = getFieldLabel(name, schema);
  const description = getFieldDescription(schema);
  
  // Determine if field has enum values (for select fields)
  const hasEnum = schema && Array.isArray(schema.enum) && schema.enum.length > 0;
  
  return (
    <FormControl 
      fullWidth 
      margin="normal" 
      error={!!validationError}
    >
      <TextField
        id={`field-${name}`}
        name={name}
        label={label}
        value={value}
        onChange={handleChange}
        onBlur={handleBlur}
        required={required}
        error={!!validationError}
        helperText={validationError || description}
        type={getInputType()}
        select={hasEnum}
        fullWidth
        variant="outlined"
        InputProps={
          schema?.format === 'password' ? {
            endAdornment: (
              <InputAdornment position="end">
                <IconButton
                  aria-label="toggle password visibility"
                  onClick={handleTogglePasswordVisibility}
                  edge="end"
                >
                  {showPassword ? <VisibilityOff /> : <Visibility />}
                </IconButton>
              </InputAdornment>
            )
          } : undefined
        }
      >
        {hasEnum && schema.enum.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </TextField>
      
     
    </FormControl>
  );
};

export default StringField;
