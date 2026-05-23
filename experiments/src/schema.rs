//! # Input Schema — Strict Validation
//!
//! Every experiment defines a schema of required inputs. The framework
//! REFUSES to run if any required field is missing, has invalid type,
//! or lacks provenance.
//!
//! This is not a toy. Missing data means invalid results. Period.

use crate::provenance::DataSource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported data types for input fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    /// Integer value.
    Integer,
    /// Floating-point value.
    Float,
    /// Text string.
    String,
    /// Boolean flag.
    Boolean,
    /// Array of floats (e.g., coordinates, weights).
    FloatArray,
    /// Array of integers (e.g., indices, counts).
    IntArray,
    /// 2D matrix of floats (e.g., distance matrix, adjacency matrix).
    FloatMatrix,
    /// JSON object (freeform structured data).
    Json,
    /// Binary data (e.g., molecular structure file).
    Binary,
}

/// A single input field in an experiment schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputField {
    /// Field name (unique within the schema).
    pub name: String,
    /// Human-readable description of what this field represents.
    pub description: String,
    /// Data type.
    pub field_type: FieldType,
    /// Whether this field is required (default: true).
    pub required: bool,
    /// Minimum value (for numeric types).
    pub min_value: Option<f64>,
    /// Maximum value (for numeric types).
    pub max_value: Option<f64>,
    /// Minimum array length (for array types).
    pub min_length: Option<usize>,
    /// Maximum array length (for array types).
    pub max_length: Option<usize>,
    /// Allowed values (for enum-like fields).
    pub allowed_values: Option<Vec<String>>,
    /// Unit of measurement (e.g., "meters", "seconds", "kelvin").
    pub unit: Option<String>,
    /// Whether data provenance is required for this field.
    pub requires_provenance: bool,
    /// Default value (as JSON string, only for optional fields).
    pub default_value: Option<String>,
}

impl InputField {
    /// Create a required float field.
    pub fn required_float(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            field_type: FieldType::Float,
            required: true,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            allowed_values: None,
            unit: None,
            requires_provenance: true,
            default_value: None,
        }
    }

    /// Create a required integer field.
    pub fn required_int(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            field_type: FieldType::Integer,
            required: true,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            allowed_values: None,
            unit: None,
            requires_provenance: true,
            default_value: None,
        }
    }

    /// Create a required float array field.
    pub fn required_float_array(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            field_type: FieldType::FloatArray,
            required: true,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            allowed_values: None,
            unit: None,
            requires_provenance: true,
            default_value: None,
        }
    }

    /// Create a required float matrix field.
    pub fn required_matrix(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            field_type: FieldType::FloatMatrix,
            required: true,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            allowed_values: None,
            unit: None,
            requires_provenance: true,
            default_value: None,
        }
    }

    /// Create a required JSON field.
    pub fn required_json(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            field_type: FieldType::Json,
            required: true,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            allowed_values: None,
            unit: None,
            requires_provenance: true,
            default_value: None,
        }
    }

    /// Create a required string field.
    pub fn required_string(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            field_type: FieldType::String,
            required: true,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            allowed_values: None,
            unit: None,
            requires_provenance: false,
            default_value: None,
        }
    }

    /// Builder: set unit.
    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    /// Builder: set min/max value bounds.
    pub fn with_bounds(mut self, min: f64, max: f64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }

    /// Builder: set min/max array length.
    pub fn with_length_bounds(mut self, min: usize, max: usize) -> Self {
        self.min_length = Some(min);
        self.max_length = Some(max);
        self
    }

    /// Builder: mark as optional with a default.
    pub fn optional_with_default(mut self, default: &str) -> Self {
        self.required = false;
        self.default_value = Some(default.to_string());
        self
    }

    /// Builder: disable provenance requirement.
    pub fn no_provenance(mut self) -> Self {
        self.requires_provenance = false;
        self
    }
}

/// The complete input schema for an experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSchema {
    /// Schema name.
    pub name: String,
    /// Schema version (for compatibility tracking).
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// All fields in the schema.
    pub fields: Vec<InputField>,
}

impl InputSchema {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: description.to_string(),
            fields: Vec::new(),
        }
    }

    /// Add a field to the schema.
    pub fn field(mut self, field: InputField) -> Self {
        self.fields.push(field);
        self
    }

    /// Get all required field names.
    pub fn required_fields(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|f| f.required)
            .map(|f| f.name.as_str())
            .collect()
    }

    /// Get a field by name.
    pub fn get_field(&self, name: &str) -> Option<&InputField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

/// An input value provided by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValue {
    /// The value as a JSON value.
    pub value: serde_json::Value,
    /// Data provenance (required for fields that demand it).
    pub source: Option<DataSource>,
}

impl InputValue {
    pub fn new(value: serde_json::Value) -> Self {
        Self { value, source: None }
    }

    pub fn with_source(mut self, source: DataSource) -> Self {
        self.source = Some(source);
        self
    }
}

/// Result of validating an input set against a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the inputs are valid.
    pub valid: bool,
    /// Errors (if any). Each error references the field name and describes the problem.
    pub errors: Vec<ValidationError>,
    /// Warnings (non-blocking issues).
    pub warnings: Vec<String>,
}

/// A single validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field name that has the error.
    pub field: String,
    /// Error description.
    pub message: String,
    /// Error category.
    pub category: ErrorCategory,
}

/// Category of validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Required field is missing.
    MissingRequired,
    /// Value has wrong type.
    TypeMismatch,
    /// Value is out of bounds.
    OutOfBounds,
    /// Array has wrong length.
    LengthViolation,
    /// Value not in allowed set.
    InvalidValue,
    /// Data provenance is required but not provided.
    MissingProvenance,
    /// Data provenance failed verification.
    ProvenanceInvalid,
}

/// Validate a set of inputs against a schema.
///
/// This is the gatekeeper. If this returns `valid: false`, the experiment
/// WILL NOT run. No exceptions. No "run anyway with defaults."
pub fn validate_inputs(
    schema: &InputSchema,
    inputs: &HashMap<String, InputValue>,
) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for field in &schema.fields {
        match inputs.get(&field.name) {
            None => {
                if field.required {
                    errors.push(ValidationError {
                        field: field.name.clone(),
                        message: format!(
                            "Required field '{}' is missing. {}",
                            field.name, field.description
                        ),
                        category: ErrorCategory::MissingRequired,
                    });
                } else if field.default_value.is_some() {
                    warnings.push(format!(
                        "Optional field '{}' not provided, using default.",
                        field.name
                    ));
                }
            }
            Some(input) => {
                // Type check
                if let Err(msg) = check_type(&field.field_type, &input.value) {
                    errors.push(ValidationError {
                        field: field.name.clone(),
                        message: format!("Type mismatch for '{}': {}", field.name, msg),
                        category: ErrorCategory::TypeMismatch,
                    });
                    continue; // Don't check further if type is wrong
                }

                // Bounds check (numeric)
                if let Some(min) = field.min_value {
                    if let Some(v) = input.value.as_f64() {
                        if v < min {
                            errors.push(ValidationError {
                                field: field.name.clone(),
                                message: format!(
                                    "'{}' value {} is below minimum {}",
                                    field.name, v, min
                                ),
                                category: ErrorCategory::OutOfBounds,
                            });
                        }
                    }
                }
                if let Some(max) = field.max_value {
                    if let Some(v) = input.value.as_f64() {
                        if v > max {
                            errors.push(ValidationError {
                                field: field.name.clone(),
                                message: format!(
                                    "'{}' value {} exceeds maximum {}",
                                    field.name, v, max
                                ),
                                category: ErrorCategory::OutOfBounds,
                            });
                        }
                    }
                }

                // Array length check
                if let Some(arr) = input.value.as_array() {
                    if let Some(min_len) = field.min_length {
                        if arr.len() < min_len {
                            errors.push(ValidationError {
                                field: field.name.clone(),
                                message: format!(
                                    "'{}' has {} elements, minimum is {}",
                                    field.name,
                                    arr.len(),
                                    min_len
                                ),
                                category: ErrorCategory::LengthViolation,
                            });
                        }
                    }
                    if let Some(max_len) = field.max_length {
                        if arr.len() > max_len {
                            errors.push(ValidationError {
                                field: field.name.clone(),
                                message: format!(
                                    "'{}' has {} elements, maximum is {}",
                                    field.name,
                                    arr.len(),
                                    max_len
                                ),
                                category: ErrorCategory::LengthViolation,
                            });
                        }
                    }
                }

                // Allowed values check
                if let Some(allowed) = &field.allowed_values {
                    if let Some(s) = input.value.as_str() {
                        if !allowed.contains(&s.to_string()) {
                            errors.push(ValidationError {
                                field: field.name.clone(),
                                message: format!(
                                    "'{}' value '{}' not in allowed set: {:?}",
                                    field.name, s, allowed
                                ),
                                category: ErrorCategory::InvalidValue,
                            });
                        }
                    }
                }

                // Provenance check
                if field.requires_provenance && input.source.is_none() {
                    errors.push(ValidationError {
                        field: field.name.clone(),
                        message: format!(
                            "'{}' requires data provenance (DOI, URL, or file hash) but none was provided. \
                             This is a real experiment — anonymous data is not accepted.",
                            field.name
                        ),
                        category: ErrorCategory::MissingProvenance,
                    });
                }
            }
        }
    }

    // Check for unexpected fields
    for key in inputs.keys() {
        if !schema.fields.iter().any(|f| f.name == *key) {
            warnings.push(format!("Unknown field '{}' provided (will be ignored).", key));
        }
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

/// Check if a JSON value matches the expected field type.
fn check_type(expected: &FieldType, value: &serde_json::Value) -> Result<(), String> {
    match expected {
        FieldType::Integer => {
            if value.is_i64() || value.is_u64() {
                Ok(())
            } else {
                Err(format!("expected integer, got {}", value_type_name(value)))
            }
        }
        FieldType::Float => {
            if value.is_f64() || value.is_i64() || value.is_u64() {
                Ok(())
            } else {
                Err(format!("expected float, got {}", value_type_name(value)))
            }
        }
        FieldType::String => {
            if value.is_string() {
                Ok(())
            } else {
                Err(format!("expected string, got {}", value_type_name(value)))
            }
        }
        FieldType::Boolean => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err(format!("expected boolean, got {}", value_type_name(value)))
            }
        }
        FieldType::FloatArray | FieldType::IntArray => {
            if value.is_array() {
                Ok(())
            } else {
                Err(format!("expected array, got {}", value_type_name(value)))
            }
        }
        FieldType::FloatMatrix => {
            if value.is_array() {
                // Check it's an array of arrays
                if let Some(rows) = value.as_array() {
                    for (i, row) in rows.iter().enumerate() {
                        if !row.is_array() {
                            return Err(format!("matrix row {} is not an array", i));
                        }
                    }
                }
                Ok(())
            } else {
                Err(format!("expected 2D array (matrix), got {}", value_type_name(value)))
            }
        }
        FieldType::Json => {
            if value.is_object() {
                Ok(())
            } else {
                Err(format!("expected JSON object, got {}", value_type_name(value)))
            }
        }
        FieldType::Binary => {
            // Binary is stored as base64 string
            if value.is_string() {
                Ok(())
            } else {
                Err(format!("expected base64 string for binary data, got {}", value_type_name(value)))
            }
        }
    }
}

fn value_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_schema() -> InputSchema {
        InputSchema::new("test", "Test schema")
            .field(InputField::required_float("temperature", "Temperature in K").with_unit("K").with_bounds(0.0, 10000.0))
            .field(InputField::required_int("num_particles", "Number of particles").with_bounds(1.0, 1e9))
            .field(InputField::required_float_array("positions", "Particle positions").with_length_bounds(1, 1000))
    }

    #[test]
    fn test_valid_inputs() {
        let schema = test_schema();
        let mut inputs = HashMap::new();
        inputs.insert("temperature".to_string(), InputValue::new(serde_json::json!(300.0))
            .with_source(DataSource::doi("10.1234/test")));
        inputs.insert("num_particles".to_string(), InputValue::new(serde_json::json!(100))
            .with_source(DataSource::doi("10.1234/test")));
        inputs.insert("positions".to_string(), InputValue::new(serde_json::json!([1.0, 2.0, 3.0]))
            .with_source(DataSource::doi("10.1234/test")));

        let result = validate_inputs(&schema, &inputs);
        assert!(result.valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_missing_required() {
        let schema = test_schema();
        let inputs = HashMap::new(); // Empty!

        let result = validate_inputs(&schema, &inputs);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 3); // All 3 required fields missing
    }

    #[test]
    fn test_missing_provenance() {
        let schema = test_schema();
        let mut inputs = HashMap::new();
        inputs.insert("temperature".to_string(), InputValue::new(serde_json::json!(300.0)));
        // No provenance!
        inputs.insert("num_particles".to_string(), InputValue::new(serde_json::json!(100))
            .with_source(DataSource::doi("10.1234/test")));
        inputs.insert("positions".to_string(), InputValue::new(serde_json::json!([1.0]))
            .with_source(DataSource::doi("10.1234/test")));

        let result = validate_inputs(&schema, &inputs);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| matches!(e.category, ErrorCategory::MissingProvenance)));
    }

    #[test]
    fn test_out_of_bounds() {
        let schema = test_schema();
        let mut inputs = HashMap::new();
        inputs.insert("temperature".to_string(), InputValue::new(serde_json::json!(-50.0))
            .with_source(DataSource::doi("10.1234/test")));
        inputs.insert("num_particles".to_string(), InputValue::new(serde_json::json!(100))
            .with_source(DataSource::doi("10.1234/test")));
        inputs.insert("positions".to_string(), InputValue::new(serde_json::json!([1.0]))
            .with_source(DataSource::doi("10.1234/test")));

        let result = validate_inputs(&schema, &inputs);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| matches!(e.category, ErrorCategory::OutOfBounds)));
    }
}
