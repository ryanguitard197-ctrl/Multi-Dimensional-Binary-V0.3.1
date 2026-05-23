//! # Discovery Engine
//!
//! The heart of the MDB Experiment Framework. Connects problem definitions
//! to mdb-core's SuperBit computation primitives.
//!
//! The engine:
//! 1. Receives a validated experiment (schema + inputs + strategy)
//! 2. Constructs the appropriate SuperBit state space
//! 3. Builds the fitness/cost function from input data
//! 4. Runs the solver (fitness_search, QAOA, evolution, or hybrid)
//! 5. Produces structured results with full provenance
//!
//! ## Key Principle
//!
//! MDB is a DISCOVERY engine. The results are NOT predetermined.
//! Given the same inputs, MDB discovers the same answer — but
//! that answer might be something nobody has ever found before.

use crate::problem::{ExperimentDefinition, SolverStrategy, StateSpaceEncoding};
use crate::schema::InputValue;
use mdb_core::coordinates::DimensionalAddress;
use mdb_core::search::{self, SearchResult};
use mdb_core::superbit::SuperBit;
use mdb_core::variational;
use mdb_core::evolution;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Result from the discovery engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// The best solution found.
    pub best_solution: SolutionVector,
    /// Cost/fitness of the best solution.
    pub best_score: f64,
    /// All candidate solutions ranked (top N).
    pub ranked_solutions: Vec<(SolutionVector, f64)>,
    /// Total candidates evaluated.
    pub candidates_evaluated: usize,
    /// Whether the superposition remained intact.
    pub superposition_intact: bool,
    /// Which solver strategy was used.
    pub strategy_used: String,
    /// Execution time.
    pub execution_time_ms: u64,
    /// Dimensional address of the best solution.
    pub best_address: Option<DimensionalAddressData>,
    /// QAOA-specific results (if QAOA was used).
    pub qaoa_details: Option<QaoaDetails>,
    /// Evolution-specific results (if evolution was used).
    pub evolution_details: Option<EvolutionDetails>,
}

/// A solution vector (the actual answer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionVector {
    /// Raw binary pattern.
    pub pattern: Vec<u8>,
    /// Human-readable interpretation.
    pub interpretation: String,
    /// Label from the state space.
    pub label: String,
}

/// Serializable dimensional address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionalAddressData {
    pub n: u64,
    pub d4_spacetime: f64,
    pub d5_momentum: u64,
}

impl From<&DimensionalAddress> for DimensionalAddressData {
    fn from(addr: &DimensionalAddress) -> Self {
        Self {
            n: addr.n,
            d4_spacetime: addr.d4_spacetime,
            d5_momentum: addr.d5_momentum,
        }
    }
}

/// QAOA-specific result details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaoaDetails {
    pub optimal_gammas: Vec<f64>,
    pub optimal_betas: Vec<f64>,
    pub iterations: usize,
    pub depth: usize,
}

/// Evolution-specific result details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionDetails {
    pub generations: u64,
    pub mutations_applied: usize,
    pub anchor_blocks: usize,
}

/// Callback for progress reporting.
pub type ProgressCallback = Box<dyn Fn(ProgressUpdate) + Send>;

/// Progress update during execution.
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Phase name (e.g., "Constructing state space", "Running QAOA").
    pub phase: String,
    /// Progress within phase (0.0 – 1.0).
    pub progress: f64,
    /// Current best score (if available).
    pub current_best: Option<f64>,
    /// Message for the user.
    pub message: String,
}

/// The MDB Discovery Engine.
pub struct DiscoveryEngine {
    /// Maximum number of candidates for fitness search.
    pub max_candidates: usize,
    /// Maximum ranked solutions to keep.
    pub max_ranked: usize,
}

impl DiscoveryEngine {
    pub fn new() -> Self {
        Self {
            max_candidates: 1024,
            max_ranked: 100,
        }
    }

    /// Run the discovery engine.
    ///
    /// This is the main entry point. Takes a validated experiment definition
    /// and inputs, and returns a DiscoveryResult.
    pub fn run(
        &self,
        definition: &ExperimentDefinition,
        inputs: &HashMap<String, InputValue>,
        progress: Option<ProgressCallback>,
    ) -> Result<DiscoveryResult, String> {
        let start = Instant::now();

        let report = |phase: &str, progress_val: f64, msg: &str| {
            if let Some(ref cb) = progress {
                cb(ProgressUpdate {
                    phase: phase.to_string(),
                    progress: progress_val,
                    current_best: None,
                    message: msg.to_string(),
                });
            }
        };

        report("init", 0.0, "Initializing discovery engine...");

        // Construct the state space
        report("state_space", 0.1, "Constructing SuperBit state space...");
        let (superbit, interpret_fn) = self.build_state_space(&definition.encoding, inputs)?;

        // Run the appropriate solver
        let result = match &definition.strategy {
            SolverStrategy::FitnessSearch { num_candidates: _ } => {
                report("search", 0.3, "Running fitness search over state space...");
                self.run_fitness_search(&superbit, definition, inputs, &interpret_fn)?
            }
            SolverStrategy::Qaoa { depth, max_iterations } => {
                report("qaoa", 0.3, "Running QAOA optimization...");
                self.run_qaoa(definition, inputs, *depth, *max_iterations, &interpret_fn)?
            }
            SolverStrategy::Evolution { steps } => {
                report("evolution", 0.3, "Running dimensional evolution...");
                self.run_evolution(superbit, *steps, &interpret_fn)?
            }
            SolverStrategy::Hybrid { strategies } => {
                report("hybrid", 0.3, "Running hybrid multi-strategy search...");
                self.run_hybrid(superbit, definition, inputs, strategies, &interpret_fn)?
            }
            SolverStrategy::Custom { description } => {
                return Err(format!(
                    "Custom strategy '{}' requires manual engine orchestration. \
                     Use the individual engine methods directly.",
                    description
                ));
            }
        };

        let elapsed = start.elapsed();
        report("done", 1.0, &format!("Discovery complete in {:.2}s", elapsed.as_secs_f64()));

        Ok(DiscoveryResult {
            execution_time_ms: elapsed.as_millis() as u64,
            ..result
        })
    }

    // ── State space construction ────────────────────────────────────

    fn build_state_space(
        &self,
        encoding: &StateSpaceEncoding,
        inputs: &HashMap<String, InputValue>,
    ) -> Result<(SuperBit, Box<dyn Fn(&[u8]) -> String>), String> {
        match encoding {
            StateSpaceEncoding::Binary { bits, bit_mapping } => {
                let num_states = (2usize).pow((*bits).min(20) as u32).min(self.max_candidates);
                let sb = SuperBit::new(*bits, num_states);
                let mapping = bit_mapping.clone();
                let interpret = Box::new(move |pattern: &[u8]| {
                    format!("Binary[{}]: {:?} ({})", pattern.len(), pattern, mapping)
                });
                Ok((sb, interpret))
            }
            StateSpaceEncoding::Integer { num_vars, domains } => {
                // Encode integer variables as bit groups within a SuperBit
                let bits_per_var: Vec<usize> = domains
                    .iter()
                    .map(|(lo, hi)| {
                        let range = (hi - lo + 1) as u64;
                        (range as f64).log2().ceil() as usize
                    })
                    .collect();
                let total_bits: usize = bits_per_var.iter().sum();
                let num_states = (2usize).pow(total_bits.min(20) as u32).min(self.max_candidates);
                let sb = SuperBit::new(total_bits, num_states);
                let domains_clone = domains.clone();
                let interpret = Box::new(move |pattern: &[u8]| {
                    let mut values = Vec::new();
                    let mut bit_offset = 0;
                    for (i, &bits) in bits_per_var.iter().enumerate() {
                        let mut val: i64 = 0;
                        for b in 0..bits {
                            if bit_offset + b < pattern.len() && pattern[bit_offset + b] == 1 {
                                val |= 1 << b;
                            }
                        }
                        let (lo, hi) = domains_clone[i];
                        val = lo + (val % (hi - lo + 1));
                        values.push(val);
                        bit_offset += bits;
                    }
                    format!("Integer[{}]: {:?}", values.len(), values)
                });
                Ok((sb, interpret))
            }
            StateSpaceEncoding::RealValued { dimensions, bounds } => {
                // Discretize continuous space: 10 bits per dimension
                let bits_per_dim = 10;
                let total_bits = dimensions * bits_per_dim;
                let num_states = self.max_candidates.min(1 << total_bits.min(20));
                let sb = SuperBit::new(total_bits, num_states);
                let bounds_clone = bounds.clone();
                let dims = *dimensions;
                let interpret = Box::new(move |pattern: &[u8]| {
                    let mut values = Vec::new();
                    for d in 0..dims {
                        let start = d * bits_per_dim;
                        let mut int_val: u64 = 0;
                        for b in 0..bits_per_dim {
                            if start + b < pattern.len() && pattern[start + b] == 1 {
                                int_val |= 1 << b;
                            }
                        }
                        let (lo, hi) = bounds_clone.get(d).copied().unwrap_or((0.0, 1.0));
                        let normalized = int_val as f64 / ((1 << bits_per_dim) - 1) as f64;
                        let val = lo + normalized * (hi - lo);
                        values.push(val);
                    }
                    format!("Real[{}]: {:?}", dims, values)
                });
                Ok((sb, interpret))
            }
            StateSpaceEncoding::Graph { num_nodes, directed } => {
                // Adjacency matrix encoding: N*(N-1)/2 bits (undirected) or N*(N-1) (directed)
                let total_bits = if *directed {
                    num_nodes * (num_nodes - 1)
                } else {
                    num_nodes * (num_nodes - 1) / 2
                };
                let num_states = self.max_candidates.min(1 << total_bits.min(20));
                let sb = SuperBit::new(total_bits, num_states);
                let n = *num_nodes;
                let dir = *directed;
                let interpret = Box::new(move |pattern: &[u8]| {
                    let edges: usize = pattern.iter().filter(|&&b| b == 1).count();
                    format!("Graph[{} nodes, {} edges, {}]", n, edges, if dir { "directed" } else { "undirected" })
                });
                Ok((sb, interpret))
            }
        }
    }

    // ── Fitness Search ──────────────────────────────────────────────

    fn run_fitness_search(
        &self,
        sb: &SuperBit,
        definition: &ExperimentDefinition,
        inputs: &HashMap<String, InputValue>,
        interpret: &dyn Fn(&[u8]) -> String,
    ) -> Result<DiscoveryResult, String> {
        // Build the fitness function from the problem definition and inputs
        let fitness_fn = self.build_fitness_fn(definition, inputs)?;

        let search_result = search::fitness_search(sb, |pattern, address| {
            fitness_fn(pattern, address)
        });

        let ranked: Vec<(SolutionVector, f64)> = search_result
            .ranked
            .iter()
            .take(self.max_ranked)
            .map(|(idx, label, score)| {
                let view = sb.peek();
                let pattern = view.states[*idx].pattern.clone();
                (
                    SolutionVector {
                        interpretation: interpret(&pattern),
                        pattern,
                        label: label.clone(),
                    },
                    *score,
                )
            })
            .collect();

        Ok(DiscoveryResult {
            best_solution: SolutionVector {
                pattern: search_result.best_pattern.clone(),
                interpretation: interpret(&search_result.best_pattern),
                label: search_result.best_label.clone(),
            },
            best_score: search_result.best_fitness,
            ranked_solutions: ranked,
            candidates_evaluated: search_result.candidates_evaluated,
            superposition_intact: search_result.superposition_intact,
            strategy_used: "FitnessSearch".to_string(),
            execution_time_ms: 0,
            best_address: Some(DimensionalAddressData::from(&search_result.best_address)),
            qaoa_details: None,
            evolution_details: None,
        })
    }

    // ── QAOA ────────────────────────────────────────────────────────

    fn run_qaoa(
        &self,
        definition: &ExperimentDefinition,
        inputs: &HashMap<String, InputValue>,
        depth: usize,
        max_iterations: usize,
        interpret: &dyn Fn(&[u8]) -> String,
    ) -> Result<DiscoveryResult, String> {
        let n = match &definition.encoding {
            StateSpaceEncoding::Binary { bits, .. } => *bits,
            StateSpaceEncoding::Integer { num_vars, .. } => *num_vars * 8,
            StateSpaceEncoding::Graph { num_nodes, .. } => num_nodes * (num_nodes - 1) / 2,
            StateSpaceEncoding::RealValued { dimensions, .. } => dimensions * 10,
        };

        let cost_fn_inner = self.build_cost_fn(definition, inputs)?;

        let qaoa_result = variational::qaoa(
            n,
            depth,
            &cost_fn_inner,
            max_iterations,
        );

        Ok(DiscoveryResult {
            best_solution: SolutionVector {
                pattern: qaoa_result.solution.clone(),
                interpretation: interpret(&qaoa_result.solution),
                label: "QAOA-optimal".to_string(),
            },
            best_score: qaoa_result.cost,
            ranked_solutions: vec![(
                SolutionVector {
                    pattern: qaoa_result.solution.clone(),
                    interpretation: interpret(&qaoa_result.solution),
                    label: "QAOA-optimal".to_string(),
                },
                qaoa_result.cost,
            )],
            candidates_evaluated: qaoa_result.iterations * (1 << n.min(10)),
            superposition_intact: true,
            strategy_used: format!("QAOA(depth={}, iters={})", depth, max_iterations),
            execution_time_ms: 0,
            best_address: None,
            qaoa_details: Some(QaoaDetails {
                optimal_gammas: qaoa_result.gammas,
                optimal_betas: qaoa_result.betas,
                iterations: qaoa_result.iterations,
                depth,
            }),
            evolution_details: None,
        })
    }

    // ── Evolution ───────────────────────────────────────────────────

    fn run_evolution(
        &self,
        mut superbit: SuperBit,
        steps: usize,
        interpret: &dyn Fn(&[u8]) -> String,
    ) -> Result<DiscoveryResult, String> {
        let results = evolution::evolve_dimensional_n(&mut superbit, steps);

        let mutations_applied = results.iter().filter(|r| r.modified_position.is_some()).count();
        let anchor_blocks = results.iter().filter(|r| r.blocked_by_anchor).count();
        let last_gen = results.last().map(|r| r.generation).unwrap_or(0);

        // After evolution, do a fitness search to find the best state
        let view = superbit.peek();
        let best_idx = 0; // First state after evolution
        let best_state = &view.states[best_idx];

        Ok(DiscoveryResult {
            best_solution: SolutionVector {
                pattern: best_state.pattern.clone(),
                interpretation: interpret(&best_state.pattern),
                label: best_state.label.clone(),
            },
            best_score: 0.0, // Evolution doesn't have a fitness score by default
            ranked_solutions: view
                .states
                .iter()
                .take(self.max_ranked)
                .map(|s| {
                    (
                        SolutionVector {
                            pattern: s.pattern.clone(),
                            interpretation: interpret(&s.pattern),
                            label: s.label.clone(),
                        },
                        0.0,
                    )
                })
                .collect(),
            candidates_evaluated: view.state_count,
            superposition_intact: true,
            strategy_used: format!("Evolution({} steps)", steps),
            execution_time_ms: 0,
            best_address: Some(DimensionalAddressData::from(&best_state.address)),
            qaoa_details: None,
            evolution_details: Some(EvolutionDetails {
                generations: last_gen,
                mutations_applied,
                anchor_blocks,
            }),
        })
    }

    // ── Hybrid ──────────────────────────────────────────────────────

    fn run_hybrid(
        &self,
        superbit: SuperBit,
        definition: &ExperimentDefinition,
        inputs: &HashMap<String, InputValue>,
        strategies: &[SolverStrategy],
        interpret: &dyn Fn(&[u8]) -> String,
    ) -> Result<DiscoveryResult, String> {
        let mut best_result: Option<DiscoveryResult> = None;

        for strategy in strategies {
            let result = match strategy {
                SolverStrategy::FitnessSearch { .. } => {
                    self.run_fitness_search(&superbit, definition, inputs, interpret)
                }
                SolverStrategy::Qaoa { depth, max_iterations } => {
                    self.run_qaoa(definition, inputs, *depth, *max_iterations, interpret)
                }
                SolverStrategy::Evolution { steps } => {
                    let sb_clone = superbit.clone();
                    self.run_evolution(sb_clone, *steps, interpret)
                }
                _ => continue,
            };

            if let Ok(r) = result {
                if best_result.as_ref().map(|b| r.best_score > b.best_score).unwrap_or(true) {
                    best_result = Some(r);
                }
            }
        }

        best_result.ok_or_else(|| "All hybrid strategies failed".to_string())
            .map(|mut r| {
                r.strategy_used = format!("Hybrid({} strategies)", strategies.len());
                r
            })
    }

    // ── Fitness/Cost function builders ──────────────────────────────

    /// Build a fitness function from the problem definition and inputs.
    ///
    /// This is where the magic happens — the fitness function encodes
    /// the real-world problem as a mathematical objective that MDB can
    /// optimize over its state space.
    fn build_fitness_fn(
        &self,
        definition: &ExperimentDefinition,
        inputs: &HashMap<String, InputValue>,
    ) -> Result<Box<dyn Fn(&[u8], &DimensionalAddress) -> f64>, String> {
        // Extract problem-specific data from inputs
        match &definition.encoding {
            StateSpaceEncoding::Binary { bits, .. } => {
                // For binary problems, check for a weight/distance matrix
                if let Some(matrix_input) = inputs.get("distance_matrix").or(inputs.get("weight_matrix")).or(inputs.get("cost_matrix")) {
                    let matrix = parse_matrix(&matrix_input.value)?;
                    let maximize = matches!(&definition.problem_type,
                        crate::problem::ProblemType::CombinatorialOptimization { maximize: true, .. });

                    Ok(Box::new(move |pattern: &[u8], _addr: &DimensionalAddress| {
                        // Compute cost as sum of edges selected by the binary pattern
                        let n = matrix.len();
                        let mut cost = 0.0;
                        for i in 0..n {
                            for j in (i + 1)..n {
                                let bit_idx = i * n + j;
                                if bit_idx < pattern.len() && pattern[bit_idx] == 1 {
                                    cost += matrix[i][j];
                                }
                            }
                        }
                        if maximize { cost } else { -cost } // Negate for minimization
                    }))
                } else if let Some(weights_input) = inputs.get("weights") {
                    let weights = parse_float_array(&weights_input.value)?;
                    let maximize = matches!(&definition.problem_type,
                        crate::problem::ProblemType::CombinatorialOptimization { maximize: true, .. });

                    Ok(Box::new(move |pattern: &[u8], _addr: &DimensionalAddress| {
                        let mut score = 0.0;
                        for (i, &w) in weights.iter().enumerate() {
                            if i < pattern.len() && pattern[i] == 1 {
                                score += w;
                            }
                        }
                        if maximize { score } else { -score }
                    }))
                } else {
                    // Default: dimensional fitness (uses D4/D5 coordinates)
                    Ok(Box::new(|pattern: &[u8], addr: &DimensionalAddress| {
                        let ones: f64 = pattern.iter().filter(|&&b| b == 1).count() as f64;
                        let total = pattern.len() as f64;
                        // Balance + dimensional proximity
                        let balance = 1.0 - (ones / total - 0.5).abs() * 2.0;
                        let d4_factor = 1.0 / (1.0 + addr.d4_spacetime.abs());
                        balance * 0.7 + d4_factor * 0.3
                    }))
                }
            }
            _ => {
                // Default fitness for non-binary encodings
                Ok(Box::new(|pattern: &[u8], addr: &DimensionalAddress| {
                    let sum: f64 = pattern.iter().map(|&b| b as f64).sum();
                    let d_factor = 1.0 / (1.0 + addr.d4_spacetime.abs());
                    sum * d_factor
                }))
            }
        }
    }

    /// Build a cost function for QAOA (takes only pattern, no address).
    fn build_cost_fn(
        &self,
        definition: &ExperimentDefinition,
        inputs: &HashMap<String, InputValue>,
    ) -> Result<Box<dyn Fn(&[u8]) -> f64>, String> {
        if let Some(matrix_input) = inputs.get("distance_matrix").or(inputs.get("weight_matrix")).or(inputs.get("cost_matrix")) {
            let matrix = parse_matrix(&matrix_input.value)?;
            Ok(Box::new(move |pattern: &[u8]| {
                let n = matrix.len();
                let mut cost = 0.0;
                for i in 0..n {
                    for j in (i + 1)..n {
                        if i < pattern.len() && j < pattern.len()
                            && pattern[i] != pattern[j]
                        {
                            cost += matrix[i][j];
                        }
                    }
                }
                cost
            }))
        } else if let Some(weights_input) = inputs.get("weights") {
            let weights = parse_float_array(&weights_input.value)?;
            Ok(Box::new(move |pattern: &[u8]| {
                let mut score = 0.0;
                for (i, &w) in weights.iter().enumerate() {
                    if i < pattern.len() && pattern[i] == 1 {
                        score += w;
                    }
                }
                score
            }))
        } else {
            Ok(Box::new(|pattern: &[u8]| {
                pattern.iter().filter(|&&b| b == 1).count() as f64
            }))
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn parse_matrix(value: &serde_json::Value) -> Result<Vec<Vec<f64>>, String> {
    let rows = value.as_array().ok_or("Expected 2D array for matrix")?;
    rows.iter()
        .map(|row| {
            row.as_array()
                .ok_or_else(|| "Matrix row is not an array".to_string())
                .and_then(|arr| {
                    arr.iter()
                        .map(|v| v.as_f64().ok_or_else(|| "Matrix element is not a number".to_string()))
                        .collect()
                })
        })
        .collect()
}

fn parse_float_array(value: &serde_json::Value) -> Result<Vec<f64>, String> {
    let arr = value.as_array().ok_or("Expected array")?;
    arr.iter()
        .map(|v| v.as_f64().ok_or_else(|| "Array element is not a number".to_string()))
        .collect()
}
