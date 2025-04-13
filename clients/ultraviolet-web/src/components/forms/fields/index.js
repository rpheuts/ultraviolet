import StringField from './StringField';
import NumberField from './NumberField';
import BooleanField from './BooleanField';
import ArrayField from './ArrayField';
import ObjectField from './ObjectField';

export {
  StringField,
  NumberField,
  BooleanField,
  ArrayField,
  ObjectField
};

/**
 * Get the appropriate field component for a schema type
 * @param {string} type - Schema type
 * @returns {React.Component} Field component
 */
export const getFieldComponentForType = (type) => {
  switch (type) {
    case 'string':
      return StringField;
    case 'number':
    case 'integer':
      return NumberField;
    case 'boolean':
      return BooleanField;
    case 'array':
      return ArrayField;
    case 'object':
      return ObjectField;
    default:
      return StringField;
  }
};
