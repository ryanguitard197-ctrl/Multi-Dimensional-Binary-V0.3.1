//! # Custom Experiment Builder
//!
//! Not limited to the 15 pre-built templates. Build any experiment with
//! any fields, any types, any constraints. This is the fully customizable
//! path for researchers who know exactly what they need.
//!
//! ## Usage
//!
//! ```rust
//! use mdb_experiments::custom::CustomExperiment;
//! use mdb_experiments::schema::{InputField, FieldType};
//! use mdb_experiments::problem::*;
//!
//! let experiment = CustomExperiment::new("My Problem")
//!     .description("...")
//!     .problem_type(ProblemType::CombinatorialOptimization { ... })
//!     .encoding(StateSpaceEncoding::Binary { bits: 64, ... })
//!     .strategy(SolverStrategy::FitnessSearch { num_candidates: 512 })
//!     .add_field(InputField::required_float("x", "My variable"))
//!     .build()
//!     .unwrap();
//! ```

use crate::problem::*;
use crate::schema::{FieldType, InputField, InputSchema};
use serde::{Deserialize, Serialize};

/// A fully customizable experiment builder.
///
/// Unlike the catalog experiments which are pre-configured, this lets you
/// define every aspect of the experiment from scratch.
#[derive(Debug, Clone)]
pub struct CustomExperiment {
    name: String,
    description: String,
    version: String,
    author: String,
    problem_type: Option<ProblemType>,
    encoding: Option<StateSpaceEncoding>,
    strategy: Option<SolverStrategy>,
    fields: Vec<InputField>,
    success_criteria: Option<SuccessCriteria>,
    tags: Vec<String>,
    references: Vec<String>,
    /// If true, all added fields require provenance by default.
    require_provenance_by_default: bool,
}

impl CustomExperiment {
    /// Start building a custom experiment.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            author: String::new(),
            problem_type: None,
            encoding: None,
            strategy: None,
            fields: Vec::new(),
            success_criteria: None,
            tags: Vec::new(),
            references: Vec::new(),
            require_provenance_by_default: true,
        }
    }

    /// Set description.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Set version.
    pub fn version(mut self, v: &str) -> Self {
        self.version = v.to_string();
        self
    }

    /// Set author.
    pub fn author(mut self, a: &str) -> Self {
        self.author = a.to_string();
        self
    }

    /// Set the problem type.
    pub fn problem_type(mut self, pt: ProblemType) -> Self {
        self.problem_type = Some(pt);
        self
    }

    /// Set the state space encoding.
    pub fn encoding(mut self, enc: StateSpaceEncoding) -> Self {
        self.encoding = Some(enc);
        self
    }

    /// Set the solver strategy.
    pub fn strategy(mut self, strat: SolverStrategy) -> Self {
        self.strategy = Some(strat);
        self
    }

    /// Set the success criteria.
    pub fn success_criteria(mut self, criteria: SuccessCriteria) -> Self {
        self.success_criteria = Some(criteria);
        self
    }

    /// Add an input field. Fields are fully configurable:
    ///
    /// ```rust
    /// .add_field(InputField::required_float("temperature", "Reaction temperature")
    ///     .with_unit("kelvin")
    ///     .with_bounds(200.0, 500.0))
    /// .add_field(InputField::required_matrix("distances", "City distance matrix")
    ///     .with_unit("km"))
    /// .add_field(InputField::required_json("config", "Solver config")
    ///     .no_provenance())
    /// .add_field(InputField::required_int("timeout", "Max seconds")
    ///     .optional_with_default("300"))
    /// ```
    pub fn add_field(mut self, field: InputField) -> Self {
        self.fields.push(field);
        self
    }

    /// Add a required float field (convenience).
    pub fn float_field(self, name: &str, description: &str) -> Self {
        self.add_field(InputField::required_float(name, description))
    }

    /// Add a required integer field (convenience).
    pub fn int_field(self, name: &str, description: &str) -> Self {
        self.add_field(InputField::required_int(name, description))
    }

    /// Add a required string field (convenience).
    pub fn string_field(self, name: &str, description: &str) -> Self {
        self.add_field(InputField::required_string(name, description))
    }

    /// Add a required float array field (convenience).
    pub fn float_array_field(self, name: &str, description: &str) -> Self {
        self.add_field(InputField::required_float_array(name, description))
    }

    /// Add a required matrix field (convenience).
    pub fn matrix_field(self, name: &str, description: &str) -> Self {
        self.add_field(InputField::required_matrix(name, description))
    }

    /// Add a required JSON field (convenience).
    pub fn json_field(self, name: &str, description: &str) -> Self {
        self.add_field(InputField::required_json(name, description))
    }

    /// Add a custom field with full control.
    pub fn custom_field(
        mut self,
        name: &str,
        description: &str,
        field_type: FieldType,
        required: bool,
        requires_provenance: bool,
    ) -> Self {
        self.fields.push(InputField {
            name: name.to_string(),
            description: description.to_string(),
            field_type,
            required,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            allowed_values: None,
            unit: None,
            requires_provenance,
            default_value: None,
        });
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Add a reference.
    pub fn reference(mut self, reference: &str) -> Self {
        self.references.push(reference.to_string());
        self
    }

    /// Whether to require provenance on all fields by default.
    /// Individual fields can still override with `.no_provenance()`.
    pub fn require_provenance(mut self, require: bool) -> Self {
        self.require_provenance_by_default = require;
        self
    }

    /// Build the experiment definition.
    ///
    /// Validates that all required components are present:
    /// - Name (always set)
    /// - Problem type
    /// - Encoding
    /// - Strategy
    /// - At least one input field
    pub fn build(self) -> Result<ExperimentDefinition, String> {
        if self.fields.is_empty() {
            return Err(
                "At least one input field is required. An experiment with no inputs \
                 is just a random number generator."
                    .to_string(),
            );
        }

        let problem_type = self.problem_type.ok_or(
            "Problem type is required. Use .problem_type() to set it.\n\
             Options: CombinatorialOptimization, ContinuousOptimization, \
             ConstraintSatisfaction, Simulation, Discovery"
        )?;

        let encoding = self.encoding.ok_or(
            "State space encoding is required. Use .encoding() to set it.\n\
             Options: Binary, Integer, RealValued, Graph"
        )?;

        let strategy = self.strategy.ok_or(
            "Solver strategy is required. Use .strategy() to set it.\n\
             Options: FitnessSearch, Qaoa, Evolution, Hybrid, Custom"
        )?;

        let success_criteria = self.success_criteria.unwrap_or(SuccessCriteria::OpenEnded {
            description: "Discover the best possible solution".to_string(),
        });

        // Build the input schema
        let schema_name = self.name.to_lowercase().replace(' ', "_");
        let mut schema = InputSchema::new(&schema_name, &self.description);
        for field in self.fields {
            schema = schema.field(field);
        }

        Ok(ExperimentDefinition {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.name,
            description: self.description,
            version: self.version,
            author: if self.author.is_empty() {
                "Custom".to_string()
            } else {
                self.author
            },
            problem_type,
            encoding,
            strategy,
            input_schema: schema,
            success_criteria,
            tags: self.tags,
            references: self.references,
        })
    }

    /// Quick-build for common patterns: optimization over a binary state space.
    pub fn binary_optimization(name: &str, bits: usize, maximize: bool) -> Self {
        Self::new(name)
            .problem_type(ProblemType::CombinatorialOptimization {
                maximize,
                objective: format!("{} objective over {} binary variables", 
                    if maximize { "Maximize" } else { "Minimize" }, bits),
            })
            .encoding(StateSpaceEncoding::Binary {
                bits,
                bit_mapping: "User-defined binary variables".to_string(),
            })
            .strategy(SolverStrategy::Hybrid {
                strategies: vec![
                    SolverStrategy::FitnessSearch { num_candidates: 512 },
                    SolverStrategy::Qaoa { depth: 3, max_iterations: 100 },
                ],
            })
    }

    /// Quick-build for common patterns: continuous optimization.
    pub fn continuous_optimization(name: &str, dimensions: usize, maximize: bool) -> Self {
        Self::new(name)
            .problem_type(ProblemType::ContinuousOptimization {
                maximize,
                dimensions,
                objective: format!("{} objective over {} continuous variables",
                    if maximize { "Maximize" } else { "Minimize" }, dimensions),
            })
            .encoding(StateSpaceEncoding::RealValued {
                dimensions,
                bounds: vec![(0.0, 1.0); dimensions],
            })
            .strategy(SolverStrategy::FitnessSearch { num_candidates: 1024 })
    }

    /// Quick-build for common patterns: open-ended discovery.
    pub fn discovery(name: &str, domain: &str, state_bits: usize) -> Self {
        Self::new(name)
            .problem_type(ProblemType::Discovery {
                domain: domain.to_string(),
                novelty_metric: "To be defined by input data".to_string(),
            })
            .encoding(StateSpaceEncoding::Binary {
                bits: state_bits,
                bit_mapping: "Discovery state space".to_string(),
            })
            .strategy(SolverStrategy::Hybrid {
                strategies: vec![
                    SolverStrategy::FitnessSearch { num_candidates: 512 },
                    SolverStrategy::Evolution { steps: 500 },
                ],
            })
            .success_criteria(SuccessCriteria::OpenEnded {
                description: format!("Novel discovery in {}", domain),
            })
    }

    /// Quick-build for common patterns: graph optimization.
    pub fn graph_optimization(name: &str, num_nodes: usize, directed: bool) -> Self {
        Self::new(name)
            .problem_type(ProblemType::CombinatorialOptimization {
                maximize: false,
                objective: format!("Optimize over {}-node {} graph",
                    num_nodes, if directed { "directed" } else { "undirected" }),
            })
            .encoding(StateSpaceEncoding::Graph {
                num_nodes,
                directed,
            })
            .strategy(SolverStrategy::Qaoa { depth: 4, max_iterations: 200 })
    }
}

/// Import/export experiment definitions as JSON.
impl ExperimentDefinition {
    /// Export this experiment definition as JSON.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))
    }

    /// Import an experiment definition from JSON.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse: {}", e))
    }

    /// Save to a file.
    pub fn save(&self, path: &str) -> Result<(), String> {
        let json = self.to_json()?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write: {}", e))
    }

    /// Load from a file.
    pub fn load(path: &str) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read: {}", e))?;
        Self::from_json(&json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_binary_optimization() {
        let exp = CustomExperiment::binary_optimization("Test Optimization", 32, true)
            .description("Test binary optimization")
            .add_field(InputField::required_matrix("weights", "Edge weights"))
            .add_field(InputField::required_int("num_nodes", "Number of nodes")
                .with_bounds(2.0, 1000.0))
            .build()
            .unwrap();

        assert_eq!(exp.name, "Test Optimization");
        assert_eq!(exp.input_schema.fields.len(), 2);
    }

    #[test]
    fn test_custom_continuous() {
        let exp = CustomExperiment::continuous_optimization("Param Tuning", 10, false)
            .description("Tuning 10 parameters")
            .float_field("target_value", "What we're trying to reach")
            .float_array_field("initial_guess", "Starting point")
            .build()
            .unwrap();

        assert_eq!(exp.input_schema.fields.len(), 2);
    }

    #[test]
    fn test_custom_discovery() {
        let exp = CustomExperiment::discovery("Novel Catalysts", "chemistry", 128)
            .add_field(InputField::required_json("element_properties", "Properties per element"))
            .add_field(InputField::required_float("target_activation_energy", "Target Ea")
                .with_unit("kJ/mol"))
            .build()
            .unwrap();

        assert!(exp.tags.is_empty()); // No tags added
        assert_eq!(exp.input_schema.fields.len(), 2);
    }

    #[test]
    fn test_refuses_empty_fields() {
        let result = CustomExperiment::new("Empty")
            .problem_type(ProblemType::Discovery {
                domain: "test".to_string(),
                novelty_metric: "test".to_string(),
            })
            .encoding(StateSpaceEncoding::Binary { bits: 8, bit_mapping: "test".to_string() })
            .strategy(SolverStrategy::FitnessSearch { num_candidates: 10 })
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("At least one input field"));
    }

    #[test]
    fn test_refuses_missing_problem_type() {
        let result = CustomExperiment::new("No Problem")
            .add_field(InputField::required_float("x", "A variable"))
            .encoding(StateSpaceEncoding::Binary { bits: 8, bit_mapping: "test".to_string() })
            .strategy(SolverStrategy::FitnessSearch { num_candidates: 10 })
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Problem type is required"));
    }

    #[test]
    fn test_json_roundtrip() {
        let exp = CustomExperiment::binary_optimization("JSON Test", 16, true)
            .description("Round-trip test")
            .float_field("weight", "A weight")
            .build()
            .unwrap();

        let json = exp.to_json().unwrap();
        let loaded = ExperimentDefinition::from_json(&json).unwrap();
        assert_eq!(exp.name, loaded.name);
        assert_eq!(exp.input_schema.fields.len(), loaded.input_schema.fields.len());
    }
}
