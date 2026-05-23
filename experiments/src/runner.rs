//! # Experiment Runner
//!
//! The top-level orchestrator that:
//! 1. Accepts an experiment definition + user inputs
//! 2. Validates EVERYTHING (refuses to run if anything is missing)
//! 3. Builds the provenance chain
//! 4. Runs the discovery engine
//! 5. Generates the reproducibility manifest
//! 6. Returns structured results
//!
//! This is the only entry point users should call.

use crate::engine::{DiscoveryEngine, DiscoveryResult, EngineParams, ProgressCallback};
use crate::manifest::ReproducibilityManifest;
use crate::problem::ExperimentDefinition;
use crate::provenance::ProvenanceChain;
use crate::schema::{validate_inputs, InputValue, ValidationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// The complete result of an experiment run.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// Whether the experiment completed successfully.
    pub success: bool,
    /// Human-readable status message.
    pub message: String,
    /// Input validation result.
    pub validation: ValidationResult,
    /// Discovery result (None if validation failed).
    pub discovery: Option<DiscoveryResult>,
    /// Reproducibility manifest (None if experiment didn't run).
    pub manifest: Option<ReproducibilityManifest>,
    /// Total wall-clock time in milliseconds.
    pub total_time_ms: u64,
}

/// The experiment runner.
pub struct ExperimentRunner {
    /// The discovery engine instance.
    engine: DiscoveryEngine,
    /// Maximum candidates for the engine.
    max_candidates: usize,
}

impl ExperimentRunner {
    pub fn new() -> Self {
        Self {
            engine: DiscoveryEngine::new(),
            max_candidates: 1024,
        }
    }

    /// Set the maximum number of candidates for the engine.
    pub fn with_max_candidates(mut self, max: usize) -> Self {
        self.max_candidates = max;
        self.engine.max_candidates = max;
        self
    }

    /// Run an experiment.
    ///
    /// This is THE entry point. Provide the experiment definition and
    /// a map of input field names to InputValues. The runner handles
    /// everything else.
    ///
    /// ## Guarantees
    ///
    /// - Will NOT run if any required input is missing
    /// - Will NOT run if any input fails type validation
    /// - Will NOT run if required provenance is missing
    /// - WILL produce a reproducibility manifest on success
    /// - WILL give you detailed error messages on failure
    pub fn run(
        &self,
        definition: &ExperimentDefinition,
        inputs: HashMap<String, InputValue>,
        progress: Option<ProgressCallback>,
    ) -> ExperimentResult {
        let start = Instant::now();

        // ── Step 1: Validate inputs ─────────────────────────────────
        log::info!(
            "Experiment '{}': validating {} inputs against schema...",
            definition.name,
            inputs.len()
        );

        let validation = validate_inputs(&definition.input_schema, &inputs);

        if !validation.valid {
            let error_summary: Vec<String> = validation
                .errors
                .iter()
                .map(|e| format!("  • [{}] {}", e.field, e.message))
                .collect();

            let message = format!(
                "REFUSED TO RUN: {} validation error(s) found.\n\
                 This is a real experiment — all required data must be provided \
                 with proper provenance.\n\nErrors:\n{}",
                validation.errors.len(),
                error_summary.join("\n")
            );

            log::error!("Experiment '{}': {}", definition.name, message);

            return ExperimentResult {
                success: false,
                message,
                validation,
                discovery: None,
                manifest: None,
                total_time_ms: start.elapsed().as_millis() as u64,
            };
        }

        // Log warnings
        for warning in &validation.warnings {
            log::warn!("Experiment '{}': {}", definition.name, warning);
        }

        // ── Step 2: Build provenance chain ──────────────────────────
        log::info!("Experiment '{}': building provenance chain...", definition.name);

        let mut provenance = ProvenanceChain::new();
        for (field_name, input) in &inputs {
            if let Some(source) = &input.source {
                provenance.add(field_name, source.clone());
            }
        }
        provenance.finalize();

        // ── Step 3: Run the discovery engine ────────────────────────
        log::info!(
            "Experiment '{}': launching discovery engine (strategy: {:?})...",
            definition.name,
            std::mem::discriminant(&definition.strategy)
        );

        let engine_result = self.engine.run(definition, &inputs, progress);

        match engine_result {
            Ok(discovery) => {
                // ── Step 4: Generate reproducibility manifest ────────
                log::info!(
                    "Experiment '{}': discovery complete (best score: {:.6}). \
                     Generating manifest...",
                    definition.name,
                    discovery.best_score
                );

                let engine_params = EngineParams {
                    max_candidates: self.max_candidates,
                    qaoa_depth: match &definition.strategy {
                        crate::problem::SolverStrategy::Qaoa { depth, .. } => Some(*depth),
                        _ => None,
                    },
                    qaoa_max_iterations: match &definition.strategy {
                        crate::problem::SolverStrategy::Qaoa { max_iterations, .. } => {
                            Some(*max_iterations)
                        }
                        _ => None,
                    },
                    evolution_steps: match &definition.strategy {
                        crate::problem::SolverStrategy::Evolution { steps } => Some(*steps),
                        _ => None,
                    },
                    random_seed: None,
                };

                let manifest = ReproducibilityManifest::build(
                    definition.clone(),
                    &inputs,
                    provenance,
                    &discovery,
                    engine_params,
                );

                let total_time = start.elapsed().as_millis() as u64;

                log::info!(
                    "Experiment '{}': COMPLETE in {}ms. Manifest ID: {}",
                    definition.name,
                    total_time,
                    manifest.manifest_id
                );

                ExperimentResult {
                    success: true,
                    message: format!(
                        "Experiment '{}' completed successfully.\n\
                         Best score: {:.6}\n\
                         Best solution: {}\n\
                         Candidates evaluated: {}\n\
                         Superposition intact: {}\n\
                         Manifest ID: {}",
                        definition.name,
                        discovery.best_score,
                        discovery.best_solution.interpretation,
                        discovery.candidates_evaluated,
                        discovery.superposition_intact,
                        manifest.manifest_id,
                    ),
                    validation,
                    discovery: Some(discovery),
                    manifest: Some(manifest),
                    total_time_ms: total_time,
                }
            }
            Err(error) => {
                log::error!(
                    "Experiment '{}': engine failed: {}",
                    definition.name,
                    error
                );

                ExperimentResult {
                    success: false,
                    message: format!(
                        "Discovery engine failed: {}\n\
                         Inputs were valid, but the engine encountered an error. \
                         This may indicate a problem with the state space encoding \
                         or solver configuration.",
                        error
                    ),
                    validation,
                    discovery: None,
                    manifest: None,
                    total_time_ms: start.elapsed().as_millis() as u64,
                }
            }
        }
    }

    /// Dry-run: validate inputs without running the experiment.
    pub fn validate_only(
        &self,
        definition: &ExperimentDefinition,
        inputs: &HashMap<String, InputValue>,
    ) -> ValidationResult {
        validate_inputs(&definition.input_schema, inputs)
    }

    /// List all required fields for an experiment.
    pub fn required_fields(definition: &ExperimentDefinition) -> Vec<(String, String, String)> {
        definition
            .input_schema
            .fields
            .iter()
            .filter(|f| f.required)
            .map(|f| {
                (
                    f.name.clone(),
                    format!("{:?}", f.field_type),
                    f.description.clone(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;
    use crate::provenance::DataSource;

    #[test]
    fn test_refuses_empty_inputs() {
        let runner = ExperimentRunner::new();
        let experiment = catalog::protein_folding();
        let inputs = HashMap::new();

        let result = runner.run(&experiment, inputs, None);
        assert!(!result.success);
        assert!(result.message.contains("REFUSED TO RUN"));
        assert!(result.discovery.is_none());
        assert!(result.manifest.is_none());
    }

    #[test]
    fn test_list_required_fields() {
        let experiment = catalog::supply_chain_optimization();
        let fields = ExperimentRunner::required_fields(&experiment);
        assert!(fields.len() >= 8, "Supply chain should have 8+ required fields");

        // Check specific fields exist
        let names: Vec<&str> = fields.iter().map(|(n, _, _)| n.as_str()).collect();
        assert!(names.contains(&"num_suppliers"));
        assert!(names.contains(&"transport_costs"));
        assert!(names.contains(&"demand"));
    }

    #[test]
    fn test_validate_only() {
        let runner = ExperimentRunner::new();
        let experiment = catalog::protein_folding();
        let inputs = HashMap::new();

        let result = runner.validate_only(&experiment, &inputs);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }
}
