/**
 * Utility functions for parsing JSON Schema
 */

/**
 * Determines the appropriate field type based on schema properties
 * @param {Object} schema - JSON Schema property definition
 * @returns {string} Field type identifier
 */
export const getFieldType = (schema) => {
  if (!schema || !schema.type) {
    return 'string'; // Default to string if type is not specified
  }

  const { type, format } = schema;

  // Handle special formats
  if (type === 'string' && format) {
    switch (format) {
      case 'date':
      case 'date-time':
        return 'date';
      case 'email':
        return 'email';
      case 'uri':
        return 'url';
      case 'password':
        return 'password';
      default:
        return 'string';
    }
  }

  // Handle basic types
  switch (type) {
    case 'string':
      return 'string';
    case 'number':
    case 'integer':
      return 'number';
    case 'boolean':
      return 'boolean';
    case 'array':
      return 'array';
    case 'object':
      return 'object';
    default:
      return 'string';
  }
};

/**
 * Extracts field constraints from schema
 * @param {Object} schema - JSON Schema property definition
 * @returns {Object} Constraints object
 */
export const getFieldConstraints = (schema) => {
  if (!schema) {
    return {};
  }

  const constraints = {};

  // String constraints
  if (schema.minLength !== undefined) constraints.minLength = schema.minLength;
  if (schema.maxLength !== undefined) constraints.maxLength = schema.maxLength;
  if (schema.pattern !== undefined) constraints.pattern = schema.pattern;

  // Number constraints
  if (schema.minimum !== undefined) constraints.min = schema.minimum;
  if (schema.maximum !== undefined) constraints.max = schema.maximum;
  if (schema.multipleOf !== undefined) constraints.step = schema.multipleOf;

  // Array constraints
  if (schema.minItems !== undefined) constraints.minItems = schema.minItems;
  if (schema.maxItems !== undefined) constraints.maxItems = schema.maxItems;
  if (schema.uniqueItems !== undefined) constraints.uniqueItems = schema.uniqueItems;

  // Enum constraints
  if (schema.enum !== undefined) constraints.enum = schema.enum;

  return constraints;
};

/**
 * Gets the default value for a field based on schema
 * @param {Object} schema - JSON Schema property definition
 * @returns {any} Default value
 */
export const getDefaultValue = (schema) => {
  if (!schema) {
    return undefined;
  }

  if (schema.default !== undefined) {
    return schema.default;
  }

  // Provide sensible defaults based on type
  switch (schema.type) {
    case 'string':
      return '';
    case 'number':
    case 'integer':
      return 0;
    case 'boolean':
      return false;
    case 'array':
      return [];
    case 'object':
      return {};
    default:
      return undefined;
  }
};

/**
 * Determines if a field is required
 * @param {string} fieldName - Name of the field
 * @param {Array} required - List of required field names
 * @returns {boolean} True if field is required
 */
export const isFieldRequired = (fieldName, required = []) => {
  return required.includes(fieldName);
};

/**
 * Gets field label from schema
 * @param {string} fieldName - Name of the field
 * @param {Object} schema - JSON Schema property definition
 * @returns {string} Field label
 */
export const getFieldLabel = (fieldName, schema) => {
  if (schema && schema.title) {
    return schema.title;
  }
  
  // Convert camelCase to Title Case
  return fieldName
    .replace(/([A-Z])/g, ' $1')
    .replace(/^./, str => str.toUpperCase());
};

/**
 * Gets field description from schema
 * @param {Object} schema - JSON Schema property definition
 * @returns {string|undefined} Field description
 */
export const getFieldDescription = (schema) => {
  return schema && schema.description;
};
