//! # MDB Experiment Framework
//!
//! A rigorous experiment engine for running real scientific experiments
//! on the MDB computing paradigm.
//!
//! ## Design Philosophy
//!
//! MDB is a **discovery engine** — it finds NEW solutions to problems that
//! are currently unsolvable or computationally intractable. Reproducibility
//! means: given the same inputs, MDB will discover the same answer.
//! The answer is NOT predetermined.
//!
//! ## Core Principles
//!
//! 1. **Strict input schemas** — Every experiment defines exactly what data
//!    is required. The framework REFUSES to run with missing inputs.
//!
//! 2. **Data provenance** — Every input must have a source (DOI, URL, hash).
//!    No anonymous data. No "test values." Real problems, real data.
//!
//! 3. **Reproducibility manifests** — Every experiment run produces a manifest
//!    that captures inputs, parameters, environment, and results so that
//!    anyone can re-run and verify.
//!
//! 4. **No toys** — The framework is designed for real-world problems:
//!    protein folding, supply chain optimization, climate modeling,
//!    drug discovery, cryptographic analysis, etc.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  Experiment Definition               │
//! │  (problem type, state space, constraints, fitness)   │
//! ├─────────────────────────────────────────────────────┤
//! │                   Input Validation                   │
//! │  (schema check, provenance verify, completeness)     │
//! ├─────────────────────────────────────────────────────┤
//! │                   Discovery Engine                   │
//! │  (fitness_search / QAOA / evolution / hybrid)        │
//! ├─────────────────────────────────────────────────────┤
//! │                   Result Analysis                    │
//! │  (scoring, ranking, dimensional properties)          │
//! ├─────────────────────────────────────────────────────┤
//! │              Reproducibility Manifest                │
//! │  (inputs hash, params, results, environment)         │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`schema`] — Input schema definitions and validation
//! - [`provenance`] — Data provenance tracking (DOI, URL, hash)
//! - [`problem`] — Problem type definitions (optimization, search, etc.)
//! - [`engine`] — The discovery engine (ties into mdb-core)
//! - [`manifest`] — Reproducibility manifest generation
//! - [`catalog`] — Pre-built experiment templates (10 from papers + 5 global problems)
//! - [`runner`] — Experiment runner with progress tracking
//! - [`custom`] — Fully customizable experiment builder (any fields, any problem)
//! - [`assistant`] — AI-powered experiment planning, auto-fill, and result explanation

pub mod assistant;
pub mod catalog;
pub mod custom;
pub mod engine;
pub mod manifest;
pub mod problem;
pub mod provenance;
pub mod runner;
pub mod schema;

// Re-exports for convenience
pub use assistant::ExperimentAssistant;
pub use custom::CustomExperiment;
pub use engine::DiscoveryEngine;
pub use manifest::ReproducibilityManifest;
pub use problem::{ExperimentDefinition, ProblemType};
pub use provenance::DataSource;
pub use runner::ExperimentRunner;
pub use schema::{InputField, InputSchema, ValidationResult};
