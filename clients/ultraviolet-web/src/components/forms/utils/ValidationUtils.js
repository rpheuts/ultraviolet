/**
 * Utility functions for form validation
 */

/**
 * Validates a string value against schema constraints
 * @param {string} value - The string value to validate
 * @param {Object} schema - JSON Schema property definition
 * @returns {string|null} Error message or null if valid
 */
export const validateString = (value, schema) => {
  if (!schema) return null;
  
  // Check if required but empty
  if (value === '' && schema.required) {
    return 'This field is required';
  }
  
  // Skip further validation if empty and not required
  if (value === '' && !schema.required) {
    return null;
  }
  
  // Check minLength
  if (schema.minLength !== undefined && value.length < schema.minLength) {
    return `Must be at least ${schema.minLength} characters`;
  }
  
  // Check maxLength
  if (schema.maxLength !== undefined && value.length > schema.maxLength) {
    return `Must be at most ${schema.maxLength} characters`;
  }
  
  // Check pattern
  if (schema.pattern !== undefined) {
    const regex = new RegExp(schema.pattern);
    if (!regex.test(value)) {
      return schema.patternError || 'Invalid format';
    }
  }
  
  // Check format
  if (schema.format) {
    switch (schema.format) {
      case 'email':
        if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) {
          return 'Invalid email address';
        }
        break;
      case 'uri':
        try {
          new URL(value);
        } catch (e) {
          return 'Invalid URL';
        }
        break;
      // Add other format validations as needed
    }
  }
  
  // Check enum
  if (schema.enum !== undefined && !schema.enum.includes(value)) {
    return `Must be one of: ${schema.enum.join(', ')}`;
  }
  
  return null;
};

/**
 * Validates a number value against schema constraints
 * @param {number} value - The number value to validate
 * @param {Object} schema - JSON Schema property definition
 * @returns {string|null} Error message or null if valid
 */
export const validateNumber = (value, schema) => {
  if (!schema) return null;
  
  // Check if value is actually a number
  if (value === '' || isNaN(value)) {
    return schema.required ? 'Please enter a valid number' : null;
  }
  
  const numValue = Number(value);
  
  // Check minimum
  if (schema.minimum !== undefined && numValue < schema.minimum) {
    return `Must be greater than or equal to ${schema.minimum}`;
  }
  
  // Check maximum
  if (schema.maximum !== undefined && numValue > schema.maximum) {
    return `Must be less than or equal to ${schema.maximum}`;
  }
  
  // Check multipleOf
  if (schema.multipleOf !== undefined) {
    const remainder = numValue % schema.multipleOf;
    if (remainder !== 0 && Math.abs(remainder - schema.multipleOf) > Number.EPSILON) {
      return `Must be a multiple of ${schema.multipleOf}`;
    }
  }
  
  // Check enum
  if (schema.enum !== undefined && !schema.enum.includes(numValue)) {
    return `Must be one of: ${schema.enum.join(', ')}`;
  }
  
  return null;
};

/**
 * Validates an array value against schema constraints
 * @param {Array} value - The array value to validate
 * @param {Object} schema - JSON Schema property definition
 * @returns {string|null} Error message or null if valid
 */
export const validateArray = (value, schema) => {
  if (!schema) return null;
  
  // Check if required but empty
  if ((!value || value.length === 0) && schema.required) {
    return 'At least one item is required';
  }
  
  // Skip further validation if empty and not required
  if (!value || value.length === 0) {
    return null;
  }
  
  // Check minItems
  if (schema.minItems !== undefined && value.length < schema.minItems) {
    return `Must have at least ${schema.minItems} items`;
  }
  
  // Check maxItems
  if (schema.maxItems !== undefined && value.length > schema.maxItems) {
    return `Must have at most ${schema.maxItems} items`;
  }
  
  // Check uniqueItems
  if (schema.uniqueItems === true) {
    const uniqueValues = new Set(value.map(JSON.stringify));
    if (uniqueValues.size !== value.length) {
      return 'All items must be unique';
    }
  }
  
  return null;
};

/**
 * Validates an object value against schema constraints
 * @param {Object} value - The object value to validate
 * @param {Object} schema - JSON Schema property definition
 * @returns {Object} Object with field names as keys and error messages as values
 */
export const validateObject = (value, schema) => {
  const errors = {};
  
  if (!schema || !schema.properties) {
    return errors;
  }
  
  const required = schema.required || [];
  
  // Check each property
  Object.entries(schema.properties).forEach(([propName, propSchema]) => {
    const propValue = value?.[propName];
    const isRequired = required.includes(propName);
    
    // Skip validation if value is undefined/null and not required
    if ((propValue === undefined || propValue === null) && !isRequired) {
      return;
    }
    
    // Validate based on property type
    let error = null;
    switch (propSchema.type) {
      case 'string':
        error = validateString(propValue || '', { ...propSchema, required: isRequired });
        break;
      case 'number':
      case 'integer':
        error = validateNumber(propValue, { ...propSchema, required: isRequired });
        break;
      case 'array':
        error = validateArray(propValue || [], { ...propSchema, required: isRequired });
        break;
      case 'object':
        const nestedErrors = validateObject(propValue || {}, propSchema);
        if (Object.keys(nestedErrors).length > 0) {
          errors[propName] = nestedErrors;
        }
        return; // Skip adding to errors since we've already added nested errors
      case 'boolean':
        // Boolean values don't typically have validation errors
        break;
      default:
        // Unknown type, no validation
    }
    
    if (error) {
      errors[propName] = error;
    }
  });
  
  return errors;
};

/**
 * Validates form data against a JSON Schema
 * @param {Object} data - Form data
 * @param {Object} schema - JSON Schema
 * @returns {Object} Validation errors object
 */
export const validateFormData = (data, schema) => {
  if (!schema || schema.type !== 'object') {
    return {};
  }
  
  return validateObject(data, schema);
};

/**
 * Checks if a form has any validation errors
 * @param {Object} errors - Validation errors object
 * @returns {boolean} True if form has errors
 */
export const hasErrors = (errors) => {
  if (!errors) return false;
  
  return Object.keys(errors).some(key => {
    const error = errors[key];
    if (typeof error === 'string') return true;
    if (typeof error === 'object') return hasErrors(error);
    return false;
  });
};
