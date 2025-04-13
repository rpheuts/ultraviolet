import React from 'react';
import { 
  FormControl, 
  FormControlLabel, 
  FormHelperText,
  Switch,
  Checkbox
} from '@mui/material';
import { getFieldLabel, getFieldDescription } from '../utils/SchemaParser';

/**
 * Boolean field component for form generation
 * @param {Object} props - Component props
 * @param {string} props.name - Field name
 * @param {Object} props.schema - JSON Schema for this field
 * @param {boolean} props.required - Whether the field is required
 * @param {boolean} props.value - Current field value
 * @param {function} props.onChange - Change handler function
 * @param {string} props.error - Error message
 * @param {boolean} props.useCheckbox - Whether to use checkbox instead of switch
 */
const BooleanField = ({ 
  name, 
  schema, 
  required = false, 
  value = false, 
  onChange, 
  error,
  useCheckbox = false
}) => {
  // Handle field change
  const handleChange = (e) => {
    if (onChange) {
      onChange(e.target.checked);
    }
  };
  
  // Get field label and description
  const label = getFieldLabel(name, schema);
  const description = getFieldDescription(schema);
  
  // Determine which component to use
  const Component = useCheckbox ? Checkbox : Switch;
  
  return (
    <FormControl 
      fullWidth 
      margin="normal" 
      error={!!error}
    >
      <FormControlLabel
        control={
          <Component
            id={`field-${name}`}
            name={name}
            checked={!!value}
            onChange={handleChange}
            color="primary"
            required={required}
          />
        }
        label={label}
      />
      
      {error ? (
        <FormHelperText error>{error}</FormHelperText>
      ) : description ? (
        <FormHelperText>{description}</FormHelperText>
      ) : null}
    </FormControl>
  );
};

export default BooleanField;
