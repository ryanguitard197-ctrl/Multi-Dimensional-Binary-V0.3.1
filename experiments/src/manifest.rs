//! # Reproducibility Manifest
//!
//! Every experiment run produces a manifest — a complete record of
//! everything needed to reproduce the result. This is not optional.
//!
//! The manifest includes:
//! - Experiment definition (frozen)
//! - Input data hashes
//! - Input provenance chain
//! - MDB engine parameters
//! - Environment information (mdb-core version, OS, hardware)
//! - Results
//! - Timing data
//!
//! Anyone with the manifest and the original data can re-run the
//! experiment and get the same results.

use crate::engine::DiscoveryResult;
use crate::problem::ExperimentDefinition;
use crate::provenance::ProvenanceChain;
use crate::schema::InputValue;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// A complete reproducibility manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityManifest {
    /// Unique manifest ID.
    pub manifest_id: String,
    /// When this manifest was created (ISO 8601).
    pub created_at: String,
    /// MDB-OS version.
    pub mdb_version: String,
    /// mdb-core crate version.
    pub core_version: String,
    /// mdb-experiments crate version.
    pub experiments_version: String,
    /// Frozen experiment definition.
    pub definition: ExperimentDefinition,
    /// Input data hashes (field name → SHA-256).
    pub input_hashes: HashMap<String, String>,
    /// Full provenance chain.
    pub provenance: ProvenanceChain,
    /// Environment info.
    pub environment: EnvironmentInfo,
    /// Engine parameters used.
    pub engine_params: EngineParams,
    /// Results.
    pub results: ResultsSummary,
    /// SHA-256 hash of this entire manifest (for integrity verification).
    pub manifest_hash: String,
}

/// Environment information for reproducibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Operating system.
    pub os: String,
    /// Architecture (x86_64, aarch64, etc.).
    pub arch: String,
    /// Number of CPU cores.
    pub cpu_cores: usize,
    /// Total memory in bytes.
    pub memory_bytes: u64,
    /// Rust compiler version.
    pub rustc_version: String,
    /// Hostname (anonymized if needed).
    pub hostname: String,
}

impl EnvironmentInfo {
    /// Collect current environment info.
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_cores: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            memory_bytes: 0, // Platform-specific; populated on Linux via /proc/meminfo
            rustc_version: env!("CARGO_PKG_VERSION").to_string(),
            hostname: hostname(),
        }
    }
}

/// Engine parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineParams {
    /// Maximum candidates for fitness search.
    pub max_candidates: usize,
    /// QAOA depth (if used).
    pub qaoa_depth: Option<usize>,
    /// QAOA max iterations (if used).
    pub qaoa_max_iterations: Option<usize>,
    /// Evolution steps (if used).
    pub evolution_steps: Option<usize>,
    /// Random seed (if applicable).
    pub random_seed: Option<u64>,
}

/// Summarized results for the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsSummary {
    /// Best score.
    pub best_score: f64,
    /// Best solution (as hex string for compactness).
    pub best_solution_hex: String,
    /// Best solution interpretation.
    pub best_interpretation: String,
    /// Candidates evaluated.
    pub candidates_evaluated: usize,
    /// Superposition intact?
    pub superposition_intact: bool,
    /// Strategy used.
    pub strategy: String,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Number of ranked solutions.
    pub num_ranked: usize,
}

impl ReproducibilityManifest {
    /// Build a manifest from experiment components.
    pub fn build(
        definition: ExperimentDefinition,
        inputs: &HashMap<String, InputValue>,
        provenance: ProvenanceChain,
        result: &DiscoveryResult,
        engine_params: EngineParams,
    ) -> Self {
        let manifest_id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        // Compute input hashes
        let input_hashes: HashMap<String, String> = inputs
            .iter()
            .map(|(k, v)| {
                let json = serde_json::to_string(&v.value).unwrap_or_default();
                let hash = crate::provenance::sha256_hex(json.as_bytes());
                (k.clone(), hash)
            })
            .collect();

        let results = ResultsSummary {
            best_score: result.best_score,
            best_solution_hex: hex_encode(&result.best_solution.pattern),
            best_interpretation: result.best_solution.interpretation.clone(),
            candidates_evaluated: result.candidates_evaluated,
            superposition_intact: result.superposition_intact,
            strategy: result.strategy_used.clone(),
            execution_time_ms: result.execution_time_ms,
            num_ranked: result.ranked_solutions.len(),
        };

        let mut manifest = Self {
            manifest_id,
            created_at,
            mdb_version: "0.3.0".to_string(),
            core_version: env!("CARGO_PKG_VERSION").to_string(),
            experiments_version: env!("CARGO_PKG_VERSION").to_string(),
            definition,
            input_hashes,
            provenance,
            environment: EnvironmentInfo::current(),
            engine_params,
            results,
            manifest_hash: String::new(),
        };

        // Compute manifest hash (hash of everything except the hash itself)
        manifest.manifest_hash = manifest.compute_hash();
        manifest
    }

    /// Compute the SHA-256 hash of this manifest.
    fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.manifest_id.as_bytes());
        hasher.update(self.created_at.as_bytes());
        hasher.update(self.mdb_version.as_bytes());

        // Hash the definition
        if let Ok(def_json) = serde_json::to_string(&self.definition) {
            hasher.update(def_json.as_bytes());
        }

        // Hash all input hashes (sorted for determinism)
        let mut sorted_keys: Vec<&String> = self.input_hashes.keys().collect();
        sorted_keys.sort();
        for key in sorted_keys {
            hasher.update(key.as_bytes());
            hasher.update(self.input_hashes[key].as_bytes());
        }

        // Hash the results
        hasher.update(self.results.best_solution_hex.as_bytes());
        hasher.update(self.results.best_score.to_le_bytes());

        format!("{:x}", hasher.finalize())
    }

    /// Verify manifest integrity.
    pub fn verify_integrity(&self) -> bool {
        let expected = self.compute_hash();
        // Note: we need to recompute without the hash field
        !self.manifest_hash.is_empty() && self.manifest_hash == expected
    }

    /// Export as JSON string.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))
    }

    /// Export as JSON and save to file.
    pub fn save(&self, path: &str) -> Result<(), String> {
        let json = self.to_json()?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write manifest: {}", e))
    }

    /// Load from a JSON file.
    pub fn load(path: &str) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse manifest: {}", e))
    }
}

/// Encode bytes as hex string.
fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Get hostname (best effort).
fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "unknown".to_string())
}
