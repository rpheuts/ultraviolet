import React, { useState, useEffect } from 'react';
import { 
  Box, 
  Button, 
  CircularProgress, 
  Paper, 
  Typography,
  Alert
} from '@mui/material';
import { Send } from '@mui/icons-material';
import { getFieldType } from './utils/SchemaParser';
import { validateFormData, hasErrors } from './utils/ValidationUtils';
import { getFieldComponentForType } from './fields';

/**
 * Form Generator component that creates a form based on JSON Schema
 * @param {Object} props - Component props
 * @param {Object} props.spectrum - Spectrum definition
 * @param {string} props.frequency - Selected frequency
 * @param {function} props.onSubmit - Submit handler function
 * @param {boolean} props.loading - Whether the form is in loading state
 * @param {string} props.error - Error message
 */
const FormGenerator = ({ 
  spectrum, 
  frequency, 
  onSubmit,
  loading = false,
  error = null
}) => {
  const [formData, setFormData] = useState({});
  const [validationErrors, setValidationErrors] = useState({});
  const [touched, setTouched] = useState(false);
  
  // Get the wavelength (method) definition from the spectrum
  const wavelength = spectrum?.wavelengths?.find(w => w.frequency === frequency);
  
  // Get the input schema
  const schema = wavelength?.input || { type: 'object', properties: {} };
  
  // Reset form when frequency changes
  useEffect(() => {
    setFormData({});
    setValidationErrors({});
    setTouched(false);
  }, [frequency]);
  
  // Handle form submission
  const handleSubmit = (e) => {
    e.preventDefault();
    
    // Validate form data
    const errors = validateFormData(formData, schema);
    setValidationErrors(errors);
    setTouched(true);
    
    // If there are no errors, submit the form
    if (!hasErrors(errors)) {
      if (onSubmit) {
        onSubmit(formData);
      }
    }
  };
  
  // Handle form field change
  const handleFieldChange = (name, value) => {
    setFormData(prevData => ({
      ...prevData,
      [name]: value
    }));
    
    // If the form has been touched, validate the field
    if (touched) {
      const fieldSchema = schema.properties?.[name];
      const fieldType = getFieldType(fieldSchema);
      
      // Validate the field based on its type
      let fieldError = null;
      
      // Update validation errors
      setValidationErrors(prevErrors => ({
        ...prevErrors,
        [name]: fieldError
      }));
    }
  };
  
  // If no frequency is selected, show a message
  if (!frequency || !wavelength) {
    return (
      <Paper sx={{ padding: 3, textAlign: 'center' }}>
        <Typography variant="body1" color="text.secondary">
          Select a frequency to generate a form.
        </Typography>
      </Paper>
    );
  }
  
  // Render form fields based on schema
  const renderFormFields = () => {
    if (!schema.properties) {
      return (
        <Typography variant="body1" color="text.secondary">
          No input parameters required.
        </Typography>
      );
    }
    
    return Object.entries(schema.properties).map(([name, propSchema]) => {
      const fieldType = getFieldType(propSchema);
      const FieldComponent = getFieldComponentForType(fieldType);
      const required = schema.required?.includes(name) || false;
      
      return (
        <FieldComponent
          key={name}
          name={name}
          schema={propSchema}
          required={required}
          value={formData[name]}
          onChange={(value) => handleFieldChange(name, value)}
          error={validationErrors[name]}
        />
      );
    });
  };
  
  return (
    <Box component="form" onSubmit={handleSubmit} noValidate>
      <Paper sx={{ padding: 3, marginBottom: 3 }}>
        <Typography variant="h6" gutterBottom>
          {wavelength.frequency}
        </Typography>
        
        {wavelength.description && (
          <Typography variant="body2" color="text.secondary" paragraph>
            {wavelength.description}
          </Typography>
        )}
        
        {error && (
          <Alert severity="error" sx={{ marginBottom: 2 }}>
            {error}
          </Alert>
        )}
        
        {renderFormFields()}
        
        <Box sx={{ marginTop: 3, display: 'flex', justifyContent: 'flex-end' }}>
          <Button
            type="submit"
            variant="contained"
            color="primary"
            startIcon={loading ? <CircularProgress size={20} color="inherit" /> : <Send />}
            disabled={loading}
          >
            {loading ? 'Sending...' : 'Submit'}
          </Button>
        </Box>
      </Paper>
    </Box>
  );
};

export default FormGenerator;
