//! # Problem Definitions
//!
//! An experiment is defined by:
//! 1. What TYPE of problem it is (optimization, search, evolution, hybrid)
//! 2. How the state space is encoded as SuperBits
//! 3. What the fitness/cost function looks like
//! 4. What constraints apply
//! 5. What constitutes a "discovery" vs. a known result
//!
//! ## Problem Types
//!
//! - **Combinatorial Optimization** — Find the best assignment from a discrete
//!   set (TSP, MaxCut, scheduling). Uses QAOA or fitness_search.
//! - **Continuous Optimization** — Find optimal parameters in continuous space.
//!   Uses dimensional search with fitness functions.
//! - **Constraint Satisfaction** — Find any assignment satisfying all constraints
//!   (SAT, graph coloring). Uses evolution + search.
//! - **Simulation** — Evolve a system forward and observe emergent behavior.
//!   Uses dimensional evolution.
//! - **Discovery** — Open-ended exploration of a state space for novel solutions.
//!   Uses hybrid approaches.

use crate::schema::InputSchema;
use serde::{Deserialize, Serialize};

/// Type of computational problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProblemType {
    /// Find the minimum/maximum of a cost/fitness function
    /// over a combinatorial (discrete) state space.
    CombinatorialOptimization {
        /// Whether to maximize (true) or minimize (false).
        maximize: bool,
        /// Description of the objective.
        objective: String,
    },
    /// Find optimal parameters in a continuous space.
    ContinuousOptimization {
        maximize: bool,
        /// Number of continuous dimensions.
        dimensions: usize,
        objective: String,
    },
    /// Find any assignment satisfying all constraints.
    ConstraintSatisfaction {
        /// Number of constraints.
        num_constraints: usize,
        /// Description of constraints.
        constraint_description: String,
    },
    /// Evolve a system and observe.
    Simulation {
        /// Number of evolution steps.
        steps: usize,
        /// What to observe/measure.
        observables: Vec<String>,
    },
    /// Open-ended discovery.
    Discovery {
        /// What domain we're exploring.
        domain: String,
        /// What novelty looks like (how to measure it).
        novelty_metric: String,
    },
}

/// How the problem maps to MDB's SuperBit state space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateSpaceEncoding {
    /// Binary encoding: each state is a binary string.
    /// Used for combinatorial problems (TSP, MaxCut, knapsack).
    Binary {
        /// Number of bits per state.
        bits: usize,
        /// Description of what each bit (or bit group) represents.
        bit_mapping: String,
    },
    /// Integer encoding: each state is a vector of integers.
    /// Used for permutation problems (TSP), scheduling.
    Integer {
        /// Number of variables.
        num_vars: usize,
        /// Domain of each variable (inclusive).
        domains: Vec<(i64, i64)>,
    },
    /// Real-valued encoding: each state has continuous coordinates.
    /// Used for molecular geometry, parameter tuning.
    RealValued {
        /// Number of dimensions.
        dimensions: usize,
        /// Bounds per dimension.
        bounds: Vec<(f64, f64)>,
    },
    /// Graph encoding: the state space IS a graph.
    /// Used for network problems, molecular graphs.
    Graph {
        /// Number of nodes.
        num_nodes: usize,
        /// Whether directed.
        directed: bool,
    },
}

/// How the MDB engine should approach the problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolverStrategy {
    /// Use mdb-core's fitness_search over a SuperBit state space.
    FitnessSearch {
        /// Number of candidate states to generate.
        num_candidates: usize,
    },
    /// Use QAOA (variational optimization).
    Qaoa {
        /// Number of QAOA layers (depth).
        depth: usize,
        /// Maximum iterations for parameter optimization.
        max_iterations: usize,
    },
    /// Use dimensional evolution.
    Evolution {
        /// Number of evolution steps.
        steps: usize,
    },
    /// Hybrid: try multiple strategies and take the best result.
    Hybrid {
        /// Strategies to try.
        strategies: Vec<SolverStrategy>,
    },
    /// Custom: user provides the engine orchestration.
    Custom {
        /// Description of the custom approach.
        description: String,
    },
}

/// A complete experiment definition.
///
/// This is the full specification of what to run. Combined with validated
/// inputs and a solver strategy, it's everything needed to execute and
/// reproduce the experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentDefinition {
    /// Unique experiment identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Detailed description of the experiment.
    pub description: String,
    /// Version of this experiment definition.
    pub version: String,
    /// Author/creator.
    pub author: String,
    /// Problem type.
    pub problem_type: ProblemType,
    /// State space encoding.
    pub encoding: StateSpaceEncoding,
    /// Solver strategy to use.
    pub strategy: SolverStrategy,
    /// Input schema (what data is needed).
    pub input_schema: InputSchema,
    /// What constitutes "success" for this experiment.
    pub success_criteria: SuccessCriteria,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Reference papers/sources for this experiment type.
    pub references: Vec<String>,
}

/// What constitutes success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuccessCriteria {
    /// Beat a known baseline value.
    BeatBaseline {
        /// The baseline value.
        baseline: f64,
        /// Description of the baseline.
        description: String,
    },
    /// Find any feasible solution (for constraint satisfaction).
    FindFeasible,
    /// Find a solution better than previous MDB runs.
    ImproveOnPrevious {
        /// ID of the previous experiment run to beat.
        previous_run_id: String,
    },
    /// Discover something novel (for open-ended exploration).
    NovelDiscovery {
        /// How novelty is measured.
        novelty_threshold: f64,
    },
    /// No specific target — the experiment IS the discovery.
    OpenEnded {
        /// What we're looking for.
        description: String,
    },
}

impl ExperimentDefinition {
    /// Create a new experiment definition builder.
    pub fn builder(name: &str) -> ExperimentBuilder {
        ExperimentBuilder {
            name: name.to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            author: "Ryan Guitard".to_string(),
            problem_type: None,
            encoding: None,
            strategy: None,
            input_schema: None,
            success_criteria: None,
            tags: Vec::new(),
            references: Vec::new(),
        }
    }
}

/// Builder for ExperimentDefinition.
pub struct ExperimentBuilder {
    name: String,
    description: String,
    version: String,
    author: String,
    problem_type: Option<ProblemType>,
    encoding: Option<StateSpaceEncoding>,
    strategy: Option<SolverStrategy>,
    input_schema: Option<InputSchema>,
    success_criteria: Option<SuccessCriteria>,
    tags: Vec<String>,
    references: Vec<String>,
}

impl ExperimentBuilder {
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn version(mut self, v: &str) -> Self {
        self.version = v.to_string();
        self
    }

    pub fn author(mut self, a: &str) -> Self {
        self.author = a.to_string();
        self
    }

    pub fn problem_type(mut self, pt: ProblemType) -> Self {
        self.problem_type = Some(pt);
        self
    }

    pub fn encoding(mut self, enc: StateSpaceEncoding) -> Self {
        self.encoding = Some(enc);
        self
    }

    pub fn strategy(mut self, strat: SolverStrategy) -> Self {
        self.strategy = Some(strat);
        self
    }

    pub fn schema(mut self, schema: InputSchema) -> Self {
        self.input_schema = Some(schema);
        self
    }

    pub fn success_criteria(mut self, criteria: SuccessCriteria) -> Self {
        self.success_criteria = Some(criteria);
        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn reference(mut self, reference: &str) -> Self {
        self.references.push(reference.to_string());
        self
    }

    pub fn build(self) -> Result<ExperimentDefinition, String> {
        Ok(ExperimentDefinition {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.name,
            description: self.description,
            version: self.version,
            author: self.author,
            problem_type: self.problem_type.ok_or("problem_type is required")?,
            encoding: self.encoding.ok_or("encoding is required")?,
            strategy: self.strategy.ok_or("strategy is required")?,
            input_schema: self.input_schema.ok_or("input_schema is required")?,
            success_criteria: self.success_criteria.ok_or("success_criteria is required")?,
            tags: self.tags,
            references: self.references,
        })
    }
}
