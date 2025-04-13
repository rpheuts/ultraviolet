import React from 'react';
import { 
  Box,
  Divider,
  FormControl, 
  FormHelperText,
  Paper,
  Typography
} from '@mui/material';
import { getFieldLabel, getFieldDescription, getFieldType, isFieldRequired } from '../utils/SchemaParser';

// Import field components
import StringField from './StringField';
import NumberField from './NumberField';
import BooleanField from './BooleanField';
import ArrayField from './ArrayField';

/**
 * Object field component for form generation
 * @param {Object} props - Component props
 * @param {string} props.name - Field name
 * @param {Object} props.schema - JSON Schema for this field
 * @param {boolean} props.required - Whether the field is required
 * @param {Object} props.value - Current field value
 * @param {function} props.onChange - Change handler function
 * @param {Object} props.errors - Validation errors object
 */
const ObjectField = ({ 
  name, 
  schema, 
  required = false, 
  value = {}, 
  onChange,
  errors = {}
}) => {
  // Get field label and description
  const label = getFieldLabel(name, schema);
  const description = getFieldDescription(schema);
  
  // Get properties from schema
  const properties = schema?.properties || {};
  const requiredProps = schema?.required || [];
  
  // Handle property change
  const handlePropertyChange = (propName, propValue) => {
    if (onChange) {
      onChange({
        ...(value || {}),
        [propName]: propValue
      });
    }
  };
  
  // Render the appropriate field component for each property
  const renderPropertyField = (propName, propSchema) => {
    const propType = getFieldType(propSchema);
    const propValue = value?.[propName];
    const propRequired = isFieldRequired(propName, requiredProps);
    const propError = errors?.[propName];
    
    const fieldProps = {
      name: `${name}.${propName}`,
      schema: propSchema,
      required: propRequired,
      value: propValue,
      onChange: (newValue) => handlePropertyChange(propName, newValue),
      error: typeof propError === 'string' ? propError : undefined
    };
    
    switch (propType) {
      case 'string':
        return <StringField key={propName} {...fieldProps} />;
      case 'number':
      case 'integer':
        return <NumberField key={propName} {...fieldProps} />;
      case 'boolean':
        return <BooleanField key={propName} {...fieldProps} />;
      case 'array':
        return <ArrayField key={propName} {...fieldProps} />;
      case 'object':
        return (
          <ObjectField 
            key={propName} 
            {...fieldProps} 
            errors={typeof propError === 'object' ? propError : {}}
          />
        );
      default:
        return <StringField key={propName} {...fieldProps} />;
    }
  };
  
  // If this is a root object (no name), render without the container
  if (!name) {
    return (
      <Box>
        {Object.entries(properties).map(([propName, propSchema]) => 
          renderPropertyField(propName, propSchema)
        )}
      </Box>
    );
  }
  
  return (
    <FormControl 
      fullWidth 
      margin="normal"
    >
      <Typography variant="subtitle1" gutterBottom>
        {label}{required ? ' *' : ''}
      </Typography>
      
      {description && (
        <FormHelperText sx={{ marginTop: -1, marginBottom: 1 }}>
          {description}
        </FormHelperText>
      )}
      
      <Paper variant="outlined" sx={{ padding: 2, marginBottom: 2 }}>
        {Object.entries(properties).map(([propName, propSchema], index, array) => (
          <React.Fragment key={propName}>
            {renderPropertyField(propName, propSchema)}
            {index < array.length - 1 && (
              <Divider sx={{ my: 2 }} />
            )}
          </React.Fragment>
        ))}
        
        {Object.keys(properties).length === 0 && (
          <Typography variant="body2" color="text.secondary" align="center">
            No properties defined for this object.
          </Typography>
        )}
      </Paper>
    </FormControl>
  );
};

export default ObjectField;
