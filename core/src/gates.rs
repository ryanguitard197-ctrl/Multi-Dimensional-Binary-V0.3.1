//! # Dimensional Gate System — Computing with Cascades
//!
//! Quantum computing has Hadamard, CNOT, Toffoli. MDB has **dimensional gates**
//! — operations that transform SuperBits by manipulating their cascade vectors,
//! state spaces, and dimensional coordinates.
//!
//! Unlike quantum gates that operate on complex amplitudes, MDB gates operate
//! on the *Fibonacci cascade* that defines each SuperBit's position in
//! dimensional space. The result is deterministic, inspectable, and
//! non-destructive.
//!
//! ## Gate Catalog
//!
//! | Gate              | Inputs | What it does                                    |
//! |-------------------|--------|-------------------------------------------------|
//! | CascadeHadamard   | 1      | Split into balanced superposition from cascade  |
//! | CascadeCNOT       | 2      | Entangle: control's state flips target's bits    |
//! | CascadePhase      | 1      | Rotate weights by φ-derived angle               |
//! | CascadeOracle     | 1      | Mark states matching a predicate                |
//! | Compose           | N      | Pipeline of gates applied sequentially           |
//!
//! ## Key difference from quantum gates
//!
//! Every gate produces a result that can be `peek()`'d without collapse.
//! You can apply a gate, inspect the full state space, compare dimensional
//! addresses, then decide whether to keep the result — something impossible
//! in quantum circuits where gates are irreversibly applied.

use crate::coordinates::DimensionalAddress;
use crate::superbit::{State, SuperBit};

/// The Golden Ratio — drives phase rotation and weight distribution.
const PHI: f64 = 1.618_033_988_749_895;

// ---------------------------------------------------------------------------
// Gate Results
// ---------------------------------------------------------------------------

/// Result of applying a gate — the transformed SuperBit plus metadata.
#[derive(Debug, Clone)]
pub struct GateResult {
    /// The transformed SuperBit.
    pub output: SuperBit,
    /// Name of the gate that produced this result.
    pub gate_name: String,
    /// Number of states in the output superposition.
    pub output_states: usize,
    /// Dimensional address of the output σ.
    pub output_address: DimensionalAddress,
}

// ---------------------------------------------------------------------------
// Cascade Hadamard — Create Balanced Superposition
// ---------------------------------------------------------------------------

/// **Cascade Hadamard Gate**: Transform a single-state SuperBit into a
/// balanced superposition of 2^k states derived from its cascade structure.
///
/// The quantum Hadamard gate puts a qubit into equal superposition of |0⟩
/// and |1⟩. The Cascade Hadamard does something more powerful: it generates
/// `2^depth` states by systematically flipping bits at positions selected
/// by the Golden Ratio φ, then assigns equal weights to all states.
///
/// `depth` controls how many positions are varied (each adds a factor of 2
/// to the state count). For a string of length n, max useful depth is n.
///
/// After applying Hadamard, `peek()` shows all states and their dimensional
/// addresses — impossible with a quantum Hadamard which destroys information
/// on measurement.
pub fn cascade_hadamard(sb: &SuperBit, depth: usize) -> GateResult {
    let n = sb.sigma.len();
    let depth = depth.min(n).min(16); // Cap at 16 to avoid state space explosion

    // Select positions using φ-rotation (same low-discrepancy sequence as cascade evolution)
    let mut positions: Vec<usize> = Vec::with_capacity(depth);
    for k in 1..=depth {
        let frac = (k as f64 * PHI).fract().abs();
        let pos = (frac * n as f64).floor() as usize % n;
        if !positions.contains(&pos) {
            positions.push(pos);
        }
    }
    let actual_depth = positions.len();
    let state_count = 1usize << actual_depth;

    // Generate all 2^actual_depth combinations
    let mut states = Vec::with_capacity(state_count);
    let equal_weight = 1.0 / state_count as f64;
    let mut weights = Vec::with_capacity(state_count);

    for combo in 0..state_count {
        let mut pattern = sb.sigma.clone();
        let mut label_parts = Vec::new();

        for (bit_idx, &pos) in positions.iter().enumerate() {
            if (combo >> bit_idx) & 1 == 1 {
                pattern[pos] ^= 1; // flip this position
                label_parts.push(format!("p{pos}"));
            }
        }

        let label = if label_parts.is_empty() {
            "base".to_string()
        } else {
            format!("flip({})", label_parts.join(","))
        };

        states.push(State { label, pattern });
        weights.push(equal_weight);
    }

    let output = SuperBit::with_states(
        sb.sigma.clone(),
        states,
        weights,
        sb.anchors.clone(),
    )
    .unwrap();

    let output_address = output.address();
    let output_states = output.state_count();

    GateResult {
        output,
        gate_name: format!("CascadeHadamard(depth={actual_depth})"),
        output_states,
        output_address,
    }
}

// ---------------------------------------------------------------------------
// Cascade CNOT — Entangle Two SuperBits
// ---------------------------------------------------------------------------

/// **Cascade CNOT Gate**: The control SuperBit's state determines how the
/// target SuperBit's bits are flipped.
///
/// In quantum computing, CNOT flips the target qubit if the control is |1⟩.
/// In MDB, CNOT creates a multi-state target where each control state
/// produces a different target configuration:
///
/// For each state in the control's state space:
/// - Compute the control state's D4 (Spacetime) scalar
/// - Use it to select positions in the target to flip
/// - Create a new target state for each control state
///
/// The result is a target SuperBit whose state space is correlated with
/// the control — true entanglement at the cascade level.
pub fn cascade_cnot(control: &SuperBit, target: &SuperBit) -> GateResult {
    let target_n = target.sigma.len();

    let mut states = Vec::with_capacity(control.state_count());
    let mut weights = Vec::with_capacity(control.state_count());

    for (i, ctrl_state) in control.states.iter().enumerate() {
        // Compute the control state's dimensional address
        let ctrl_addr = DimensionalAddress::from_bits(&ctrl_state.pattern);
        // Combine D4 (geometric) and D5 (fingerprint) for unique selection
        let d4_scalar = ctrl_addr.d4_spacetime;
        let d5_seed = ctrl_addr.d5_momentum as f64;

        // Use D4+D5 to select flip positions in target
        let mut target_pattern = target.sigma.clone();
        let flip_count = ((d4_scalar.abs() * PHI).fract() * target_n as f64)
            .floor() as usize;
        let flip_count = flip_count.max(1).min(target_n);

        for k in 0..flip_count {
            // Mix in d5_seed to ensure different patterns → different flips
            let frac = ((k as f64 + d4_scalar + d5_seed * 1e-15) * PHI).fract().abs();
            let pos = (frac * target_n as f64).floor() as usize % target_n;
            if !target.anchors.is_anchored(pos) {
                target_pattern[pos] ^= 1;
            }
        }

        states.push(State {
            label: format!("ctrl_{}:{}", i, ctrl_state.label),
            pattern: target_pattern,
        });
        weights.push(control.weights[i]);
    }

    let output = SuperBit::with_states(
        target.sigma.clone(),
        states,
        weights,
        target.anchors.clone(),
    )
    .unwrap();

    let output_address = output.address();
    let output_states = output.state_count();

    GateResult {
        output,
        gate_name: "CascadeCNOT".to_string(),
        output_states,
        output_address,
    }
}

// ---------------------------------------------------------------------------
// Cascade Phase — Rotate Weights by φ
// ---------------------------------------------------------------------------

/// **Cascade Phase Gate**: Rotate the probability weights of a SuperBit's
/// states using the Golden Ratio φ.
///
/// Each weight is shifted by a φ-derived rotation:
/// ```text
/// w_i' = w_i × (1 + amplitude × sin(i × φ × π))
/// ```
/// then renormalized. This redistributes probability mass in a way that's
/// driven by the same φ that governs the dimensional cascade.
///
/// `amplitude` (0.0–1.0) controls how much the weights are rotated.
/// At 0.0 the gate is identity; at 1.0 it's maximum rotation.
pub fn cascade_phase(sb: &SuperBit, amplitude: f64) -> GateResult {
    let amplitude = amplitude.clamp(0.0, 1.0);

    let mut new_weights: Vec<f64> = sb
        .weights
        .iter()
        .enumerate()
        .map(|(i, &w)| {
            let rotation = 1.0 + amplitude * (i as f64 * PHI * std::f64::consts::PI).sin();
            (w * rotation).max(1e-15) // keep positive
        })
        .collect();

    // Renormalize
    let sum: f64 = new_weights.iter().sum();
    for w in &mut new_weights {
        *w /= sum;
    }

    let output = SuperBit::with_states(
        sb.sigma.clone(),
        sb.states.clone(),
        new_weights,
        sb.anchors.clone(),
    )
    .unwrap();

    let output_address = output.address();
    let output_states = output.state_count();

    GateResult {
        output,
        gate_name: format!("CascadePhase(amp={amplitude:.2})"),
        output_states,
        output_address,
    }
}

// ---------------------------------------------------------------------------
// Cascade Oracle — Mark States Matching a Predicate
// ---------------------------------------------------------------------------

/// **Cascade Oracle Gate**: Amplify the weights of states that satisfy a
/// predicate function.
///
/// In quantum computing, the oracle marks states by flipping their phase.
/// In MDB, the oracle *boosts the weights* of matching states by a factor,
/// then renormalizes. This makes matching states more likely to be selected
/// in a collapse while keeping all states visible via `peek()`.
///
/// `predicate` takes a state pattern and returns true if it matches.
/// `boost` is the amplification factor (>1 amplifies, <1 suppresses).
pub fn cascade_oracle<F>(sb: &SuperBit, predicate: F, boost: f64) -> GateResult
where
    F: Fn(&[u8]) -> bool,
{
    let mut new_weights: Vec<f64> = sb
        .weights
        .iter()
        .enumerate()
        .map(|(i, &w)| {
            if predicate(&sb.states[i].pattern) {
                (w * boost).max(1e-15)
            } else {
                w.max(1e-15)
            }
        })
        .collect();

    // Renormalize
    let sum: f64 = new_weights.iter().sum();
    for w in &mut new_weights {
        *w /= sum;
    }

    let output = SuperBit::with_states(
        sb.sigma.clone(),
        sb.states.clone(),
        new_weights,
        sb.anchors.clone(),
    )
    .unwrap();

    let output_address = output.address();
    let output_states = output.state_count();

    GateResult {
        output,
        gate_name: format!("CascadeOracle(boost={boost:.2})"),
        output_states,
        output_address,
    }
}

// ---------------------------------------------------------------------------
// Gate Composition — Pipeline of Gates
// ---------------------------------------------------------------------------

/// A composable pipeline of gate operations.
///
/// Build a sequence of gates and apply them to a SuperBit in order.
/// The output of each gate feeds into the next.
pub struct GatePipeline {
    /// Human-readable name for this pipeline.
    pub name: String,
    /// The gate operations in order. Each takes a SuperBit and returns a GateResult.
    operations: Vec<Box<dyn Fn(&SuperBit) -> GateResult>>,
}

impl GatePipeline {
    /// Create a new empty pipeline.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            operations: Vec::new(),
        }
    }

    /// Add a Hadamard gate to the pipeline.
    pub fn hadamard(mut self, depth: usize) -> Self {
        self.operations
            .push(Box::new(move |sb| cascade_hadamard(sb, depth)));
        self
    }

    /// Add a Phase gate to the pipeline.
    pub fn phase(mut self, amplitude: f64) -> Self {
        self.operations
            .push(Box::new(move |sb| cascade_phase(sb, amplitude)));
        self
    }

    /// Add an Oracle gate to the pipeline with a static predicate.
    pub fn oracle(mut self, predicate: fn(&[u8]) -> bool, boost: f64) -> Self {
        self.operations.push(Box::new(move |sb| {
            cascade_oracle(sb, predicate, boost)
        }));
        self
    }

    /// Execute the pipeline on a SuperBit, returning the final result.
    ///
    /// Also returns intermediate results for debugging / inspection.
    pub fn execute(&self, input: &SuperBit) -> (GateResult, Vec<GateResult>) {
        let mut current = input.clone();
        let mut intermediates = Vec::new();

        for op in &self.operations {
            let result = op(&current);
            current = result.output.clone();
            intermediates.push(result);
        }

        let final_result = GateResult {
            output_address: current.address(),
            output_states: current.state_count(),
            output: current,
            gate_name: format!("Pipeline({})", self.name),
        };

        (final_result, intermediates)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::DefinitionsList;

    // -------------------------------------------------------------------
    // Cascade Hadamard Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_hadamard_creates_superposition() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1, 0, 1, 0]);
        assert_eq!(sb.state_count(), 1);

        let result = cascade_hadamard(&sb, 3);
        // 3 positions varied → 2^3 = 8 states (if all positions are unique)
        assert!(result.output_states >= 4, "should create multiple states");

        // All weights should be equal
        let first_weight = result.output.weights[0];
        for &w in &result.output.weights {
            assert!(
                (w - first_weight).abs() < 1e-10,
                "Hadamard should produce equal weights"
            );
        }
    }

    #[test]
    fn test_hadamard_preserves_sigma() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0]);
        let result = cascade_hadamard(&sb, 2);
        // σ should be unchanged (base state)
        assert_eq!(result.output.sigma, sb.sigma);
    }

    #[test]
    fn test_hadamard_base_state_present() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0]);
        let result = cascade_hadamard(&sb, 2);
        // The base (unflipped) state should be present
        let has_base = result.output.states.iter().any(|s| s.label == "base");
        assert!(has_base, "base state should be in the superposition");
    }

    #[test]
    fn test_hadamard_depth_1() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0]);
        let result = cascade_hadamard(&sb, 1);
        assert_eq!(result.output_states, 2); // base + 1 flip = 2
    }

    #[test]
    fn test_hadamard_peekable() {
        // The whole point: after Hadamard, we can peek at all states
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1, 0]);
        let result = cascade_hadamard(&sb, 3);
        let view = result.output.peek();
        assert!(view.state_count >= 4);
        // Every state should have a valid dimensional address
        for sv in &view.states {
            assert!(sv.address.n > 0);
        }
    }

    // -------------------------------------------------------------------
    // Cascade CNOT Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_cnot_state_correlation() {
        let control = SuperBit::from_bits(vec![1, 0, 1, 0]);
        let target = SuperBit::from_bits(vec![0, 0, 0, 0]);

        let result = cascade_cnot(&control, &target);
        // Single control state → single target state
        assert_eq!(result.output_states, 1);
        // Target should be modified (not all zeros anymore)
        let target_pattern = &result.output.states[0].pattern;
        let has_ones = target_pattern.iter().any(|&b| b == 1);
        assert!(has_ones, "CNOT should flip some target bits");
    }

    #[test]
    fn test_cnot_multi_control() {
        // Create control with explicitly different states
        let states = vec![
            State { label: "ctrl_a".into(), pattern: vec![1, 1, 1, 1, 0, 0, 0, 0] },
            State { label: "ctrl_b".into(), pattern: vec![0, 0, 0, 0, 1, 1, 1, 1] },
        ];
        let control = SuperBit::with_states(
            vec![1, 0, 1, 0, 1, 0, 1, 0],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();
        let target = SuperBit::from_bits(vec![0, 0, 0, 0, 0, 0, 0, 0]);

        let result = cascade_cnot(&control, &target);
        assert_eq!(result.output_states, 2);
        // Very different control patterns → different target configurations
        let p0 = &result.output.states[0].pattern;
        let p1 = &result.output.states[1].pattern;
        assert_ne!(p0, p1, "different control states → different targets");
    }

    #[test]
    fn test_cnot_preserves_target_sigma() {
        let control = SuperBit::from_bits(vec![1, 0, 1, 0]);
        let target = SuperBit::from_bits(vec![0, 1, 0, 1]);
        let result = cascade_cnot(&control, &target);
        // σ should be the original target's σ
        assert_eq!(result.output.sigma, target.sigma);
    }

    // -------------------------------------------------------------------
    // Cascade Phase Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_phase_zero_is_identity() {
        let states = vec![
            State { label: "a".into(), pattern: vec![1, 0] },
            State { label: "b".into(), pattern: vec![0, 1] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0],
            states,
            vec![0.6, 0.4],
            DefinitionsList::new(),
        ).unwrap();

        let result = cascade_phase(&sb, 0.0);
        // Weights should be unchanged
        for (original, rotated) in sb.weights.iter().zip(result.output.weights.iter()) {
            assert!(
                (original - rotated).abs() < 1e-10,
                "phase(0.0) should be identity"
            );
        }
    }

    #[test]
    fn test_phase_preserves_normalization() {
        let states = vec![
            State { label: "a".into(), pattern: vec![1, 0] },
            State { label: "b".into(), pattern: vec![0, 1] },
            State { label: "c".into(), pattern: vec![1, 1] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1],
            states,
            vec![0.5, 0.3, 0.2],
            DefinitionsList::new(),
        ).unwrap();

        let result = cascade_phase(&sb, 0.7);
        let sum: f64 = result.output.weights.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-10,
            "weights must sum to 1.0 after phase rotation"
        );
    }

    #[test]
    fn test_phase_changes_weights() {
        let states = vec![
            State { label: "a".into(), pattern: vec![1, 0] },
            State { label: "b".into(), pattern: vec![0, 1] },
            State { label: "c".into(), pattern: vec![1, 1] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1],
            states,
            vec![0.4, 0.3, 0.3],
            DefinitionsList::new(),
        ).unwrap();

        let result = cascade_phase(&sb, 1.0);
        // At least one weight should have changed
        let changed = sb
            .weights
            .iter()
            .zip(result.output.weights.iter())
            .any(|(a, b)| (a - b).abs() > 1e-10);
        assert!(changed, "phase(1.0) should change at least one weight");
    }

    // -------------------------------------------------------------------
    // Cascade Oracle Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_oracle_boosts_matching() {
        let states = vec![
            State { label: "match".into(), pattern: vec![1, 1, 1, 1] },
            State { label: "no_match".into(), pattern: vec![0, 0, 0, 0] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1, 0],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();

        // Boost states with all-1s pattern
        let result = cascade_oracle(&sb, |p| p.iter().all(|&b| b == 1), 10.0);

        // "match" state should have higher weight than "no_match"
        assert!(
            result.output.weights[0] > result.output.weights[1],
            "oracle should boost matching states"
        );
        // Sum should still be 1.0
        let sum: f64 = result.output.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_oracle_no_match_unchanged() {
        let states = vec![
            State { label: "a".into(), pattern: vec![1, 0] },
            State { label: "b".into(), pattern: vec![0, 1] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();

        // Predicate that matches nothing
        let result = cascade_oracle(&sb, |_| false, 10.0);
        // Weights should be equal (both unboosted, renormalized back to 0.5/0.5)
        assert!(
            (result.output.weights[0] - result.output.weights[1]).abs() < 1e-10,
            "no matches → equal weights"
        );
    }

    // -------------------------------------------------------------------
    // Pipeline Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_pipeline_hadamard_then_phase() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1, 0, 1, 0]);

        let pipeline = GatePipeline::new("test")
            .hadamard(3)
            .phase(0.5);

        let (final_result, intermediates) = pipeline.execute(&sb);

        assert_eq!(intermediates.len(), 2);
        assert!(final_result.output_states >= 4);
        // Weights should no longer be equal after phase
        let w = &final_result.output.weights;
        let all_equal = w.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 1e-10);
        assert!(!all_equal, "phase should break Hadamard's equal weights");
    }

    #[test]
    fn test_pipeline_full_search() {
        // Hadamard → Oracle → Oracle (amplification)
        fn has_any_ones(pattern: &[u8]) -> bool {
            pattern.iter().any(|&b| b == 1)
        }

        let sb = SuperBit::from_bits(vec![0, 0, 0, 0, 0, 0, 0, 0]);

        let pipeline = GatePipeline::new("grover_search")
            .hadamard(4)
            .oracle(has_any_ones, 10.0)
            .oracle(has_any_ones, 10.0);

        let (result, _) = pipeline.execute(&sb);
        assert!(result.output_states >= 2);

        // After two oracle boosts, matching states (with any 1s) should
        // have much higher weight than the all-zeros base state.
        let view = result.output.peek();

        let base_weight = view
            .states
            .iter()
            .find(|s| s.label == "base")
            .map(|s| s.weight)
            .unwrap_or(1.0);

        let max_flipped_weight = view
            .states
            .iter()
            .filter(|s| s.label != "base")
            .map(|s| s.weight)
            .fold(0.0f64, f64::max);

        assert!(
            max_flipped_weight > base_weight,
            "oracle-boosted states should outweigh base: {max_flipped_weight} vs {base_weight}"
        );
    }
}
