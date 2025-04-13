import React, { useState } from 'react';
import { 
  TextField, 
  FormControl, 
  FormHelperText,
  Slider
} from '@mui/material';
import { validateNumber } from '../utils/ValidationUtils';
import { getFieldLabel, getFieldDescription, getFieldConstraints } from '../utils/SchemaParser';

/**
 * Number field component for form generation
 * @param {Object} props - Component props
 * @param {string} props.name - Field name
 * @param {Object} props.schema - JSON Schema for this field
 * @param {boolean} props.required - Whether the field is required
 * @param {number|string} props.value - Current field value
 * @param {function} props.onChange - Change handler function
 * @param {function} props.onBlur - Blur handler function
 * @param {string} props.error - Error message
 */
const NumberField = ({ 
  name, 
  schema, 
  required = false, 
  value = '', 
  onChange, 
  onBlur,
  error
}) => {
  const [touched, setTouched] = useState(false);
  
  // Get field constraints
  const constraints = getFieldConstraints(schema);
  const { min, max, step } = constraints;
  
  // Determine if we should use a slider
  const useSlider = min !== undefined && max !== undefined && 
                   max - min <= 100; // Only use slider for reasonable ranges
  
  // Handle field change
  const handleChange = (e) => {
    if (onChange) {
      const newValue = e.target.value;
      onChange(newValue === '' ? '' : Number(newValue));
    }
  };
  
  // Handle slider change
  const handleSliderChange = (_, newValue) => {
    if (onChange) {
      onChange(newValue);
    }
  };
  
  // Handle field blur
  const handleBlur = (e) => {
    setTouched(true);
    if (onBlur) {
      onBlur(e);
    }
  };
  
  // Get validation error
  const validationError = touched ? (error || validateNumber(value, { ...schema, required })) : null;
  
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
      {useSlider ? (
        <>
          <FormHelperText>{label}{required ? ' *' : ''}</FormHelperText>
          <Slider
            value={value === '' ? (min || 0) : Number(value)}
            onChange={handleSliderChange}
            onBlur={handleBlur}
            aria-labelledby={`slider-${name}`}
            valueLabelDisplay="auto"
            step={step || 1}
            min={min}
            max={max}
            marks={[
              { value: min, label: min },
              { value: max, label: max }
            ]}
          />
          <TextField
            id={`field-${name}`}
            name={name}
            value={value}
            onChange={handleChange}
            onBlur={handleBlur}
            type="number"
            inputProps={{
              min,
              max,
              step: step || 1
            }}
            size="small"
            error={!!validationError}
            helperText={validationError}
            variant="outlined"
          />
        </>
      ) : (
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
          type="number"
          inputProps={{
            min,
            max,
            step: step || (schema?.type === 'integer' ? 1 : 'any')
          }}
          select={hasEnum}
          fullWidth
          variant="outlined"
        >
          {hasEnum && schema.enum.map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </TextField>
      )}
      
      {!validationError && description && !useSlider && (
        <FormHelperText>{description}</FormHelperText>
      )}
    </FormControl>
  );
};

export default NumberField;
