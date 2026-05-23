//! # Evolution — The Execution Primitive
//!
//! In MDB, the classical CPU cycle is replaced by the **evolution step**.
//! A SuperBit doesn't execute instructions — it *evolves* according to
//! deterministic rules derived from its dimensional coordinates.
//!
//! Three modes of evolution:
//!
//! 1. **Dimensional Evolution** — The raw binary string transforms based on
//!    its D3 (temporal) coordinate. For odd-length strings, the middle bit
//!    flips; for even-length strings, the last bit flips. Protected positions
//!    (anchors from the DefinitionsList) are never modified.
//!
//! 2. **Learning Evolution** — Probabilities (W) are reweighted based on
//!    observed outcomes and rewards. The SuperBit *learns*. The updated state
//!    is re-encoded into the binary string.
//!
//! 3. **Cascade Evolution** — Uses the Golden Ratio φ (which emerges from the
//!    Fibonacci dimensional cascade) to drive a non-periodic traversal of bit
//!    positions. Unlike dimensional evolution's period-2 toggle, cascade
//!    evolution visits every position in a maximally-distributed pattern,
//!    mirroring the same φ-driven growth seen in sunflower spirals and
//!    phyllotactic leaf arrangement.

use crate::superbit::SuperBit;

/// Result of an evolution step.
#[derive(Debug)]
pub struct EvolutionResult {
    /// The position that was modified (if any).
    pub modified_position: Option<usize>,
    /// Whether the evolution was blocked by an anchor.
    pub blocked_by_anchor: bool,
    /// The new generation number.
    pub generation: u64,
}

/// Perform a **dimensional evolution** step on a SuperBit.
///
/// Rules (from the formal spec):
/// - If D3 (length) is odd: flip the middle bit at position ⌊n/2⌋
/// - If D3 (length) is even: flip the last bit at position n-1
/// - Anchored positions (DefinitionsList) are NEVER modified
///
/// The generation counter increments after each evolution.
pub fn evolve_dimensional(sb: &mut SuperBit) -> EvolutionResult {
    let n = sb.sigma.len();
    if n == 0 {
        sb.generation += 1;
        return EvolutionResult {
            modified_position: None,
            blocked_by_anchor: false,
            generation: sb.generation,
        };
    }

    // Determine target position based on D3 parity
    let target = if n % 2 == 1 { n / 2 } else { n - 1 };

    // Check if the position is anchored
    let blocked = sb.anchors.is_anchored(target);

    if !blocked {
        // Flip the bit
        sb.sigma[target] ^= 1;
    }

    sb.generation += 1;

    EvolutionResult {
        modified_position: if blocked { None } else { Some(target) },
        blocked_by_anchor: blocked,
        generation: sb.generation,
    }
}

/// Perform a **learning evolution** step on a SuperBit.
///
/// Given an observed state index and a reward value, the SuperBit adjusts
/// its probability weights to favor (positive reward) or disfavor (negative
/// reward) that state. The weights are then renormalized.
///
/// ```text
/// W'ⱼ = Wⱼ + r  if j = i  else  Wⱼ
/// W' normalized so Σw'ⱼ = 1
/// G' = G + 1
/// ```
///
/// Returns the old and new weight of the reinforced state.
pub fn evolve_learning(
    sb: &mut SuperBit,
    state_index: usize,
    reward: f64,
) -> Result<(f64, f64), EvolutionError> {
    if state_index >= sb.states.len() {
        return Err(EvolutionError::StateIndexOutOfBounds(
            state_index,
            sb.states.len(),
        ));
    }

    let old_weight = sb.weights[state_index];

    // Apply reward
    sb.weights[state_index] += reward;

    // Clamp to non-negative
    for w in &mut sb.weights {
        if *w < 0.0 {
            *w = 0.0;
        }
    }

    // Renormalize
    sb.normalize_weights();

    sb.generation += 1;

    let new_weight = sb.weights[state_index];
    Ok((old_weight, new_weight))
}

/// Run multiple dimensional evolution steps.
pub fn evolve_dimensional_n(sb: &mut SuperBit, steps: usize) -> Vec<EvolutionResult> {
    (0..steps).map(|_| evolve_dimensional(sb)).collect()
}

// ---------------------------------------------------------------------------
// Cascade Evolution — φ-driven, non-periodic
// ---------------------------------------------------------------------------

/// The Golden Ratio, emergent from the Fibonacci cascade.
const PHI: f64 = 1.618_033_988_749_895;

/// Perform a **cascade evolution** step on a SuperBit.
///
/// Instead of the simple parity-based bit flip, cascade evolution uses
/// the Golden Ratio φ — the same constant that emerges from the dimensional
/// cascade (D(k)/D(k-1) → φ) — to select which bit position evolves.
///
/// The target position is:
/// ```text
/// target = floor( fract((generation + 1) × φ) × n )
/// ```
///
/// This is a *low-discrepancy sequence* (Fibonacci hashing). It visits
/// every position in the most uniformly distributed way possible, never
/// falling into the period-2 trap of simple dimensional evolution.
///
/// The same golden-angle pattern appears in sunflower seed spirals and
/// leaf phyllotaxis — nature's optimal packing driven by the same φ
/// that governs MDB's dimensional cascade.
pub fn evolve_cascade(sb: &mut SuperBit) -> EvolutionResult {
    let n = sb.sigma.len();
    if n == 0 {
        sb.generation += 1;
        return EvolutionResult {
            modified_position: None,
            blocked_by_anchor: false,
            generation: sb.generation,
        };
    }

    // Golden-ratio rotation: (generation+1) × φ, take fractional part, scale to n
    let g = (sb.generation + 1) as f64;
    let frac = (g * PHI).fract();
    // fract() can return negative for negative inputs; abs to be safe
    let frac = frac.abs();
    let target = (frac * n as f64).floor() as usize % n;

    let blocked = sb.anchors.is_anchored(target);

    if !blocked {
        sb.sigma[target] ^= 1;
    }

    sb.generation += 1;

    EvolutionResult {
        modified_position: if blocked { None } else { Some(target) },
        blocked_by_anchor: blocked,
        generation: sb.generation,
    }
}

/// Run multiple cascade evolution steps.
pub fn evolve_cascade_n(sb: &mut SuperBit, steps: usize) -> Vec<EvolutionResult> {
    (0..steps).map(|_| evolve_cascade(sb)).collect()
}

// ---------------------------------------------------------------------------
// Non-Destructive Evolution Preview
// ---------------------------------------------------------------------------

/// Non-destructive evolution preview: returns what the SuperBit **would**
/// look like after one dimensional evolution step, without modifying the
/// original.
///
/// Returns `(evolved_fork, result)`. The original SuperBit is untouched.
/// Use this to compare pre- and post-evolution states side by side.
pub fn evolve_dimensional_preview(sb: &SuperBit) -> (SuperBit, EvolutionResult) {
    let mut fork = sb.fork();
    let result = evolve_dimensional(&mut fork);
    (fork, result)
}

/// Non-destructive cascade evolution preview.
///
/// Returns `(evolved_fork, result)`. The original SuperBit is untouched.
pub fn evolve_cascade_preview(sb: &SuperBit) -> (SuperBit, EvolutionResult) {
    let mut fork = sb.fork();
    let result = evolve_cascade(&mut fork);
    (fork, result)
}

/// Errors that can occur during evolution.
#[derive(Debug, Clone, PartialEq)]
pub enum EvolutionError {
    StateIndexOutOfBounds(usize, usize),
}

impl std::fmt::Display for EvolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StateIndexOutOfBounds(idx, len) => {
                write!(f, "state index {} out of bounds (state count: {})", idx, len)
            }
        }
    }
}

impl std::error::Error for EvolutionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::DefinitionsList;
    use crate::superbit::{State, SuperBit};

    #[test]
    fn test_dimensional_evolution_odd_length() {
        // Length 5 (odd) → flip middle (position 2)
        let mut sb = SuperBit::from_bits(vec![1, 0, 0, 1, 1]);
        let result = evolve_dimensional(&mut sb);
        assert_eq!(result.modified_position, Some(2));
        assert!(!result.blocked_by_anchor);
        assert_eq!(sb.sigma[2], 1); // 0 → 1
        assert_eq!(sb.generation, 1);
    }

    #[test]
    fn test_dimensional_evolution_even_length() {
        // Length 4 (even) → flip last (position 3)
        let mut sb = SuperBit::from_bits(vec![1, 0, 1, 0]);
        let result = evolve_dimensional(&mut sb);
        assert_eq!(result.modified_position, Some(3));
        assert_eq!(sb.sigma[3], 1); // 0 → 1
    }

    #[test]
    fn test_dimensional_evolution_blocked_by_anchor() {
        // Length 5 (odd) → target position 2, but it's anchored
        let mut sb = SuperBit::from_bits(vec![1, 0, 0, 1, 1]);
        sb.anchors.add(2);
        let result = evolve_dimensional(&mut sb);
        assert_eq!(result.modified_position, None);
        assert!(result.blocked_by_anchor);
        assert_eq!(sb.sigma[2], 0); // unchanged!
        assert_eq!(sb.generation, 1); // generation still increments
    }

    #[test]
    fn test_dimensional_evolution_double_flip_restores() {
        // Two dimensional evolutions on the same position should restore original
        let original = vec![1, 0, 1, 1, 0];
        let mut sb = SuperBit::from_bits(original.clone());
        evolve_dimensional(&mut sb);
        evolve_dimensional(&mut sb);
        assert_eq!(sb.sigma, original);
        assert_eq!(sb.generation, 2);
    }

    #[test]
    fn test_learning_evolution() {
        let states = vec![
            State { label: "a".into(), pattern: vec![0] },
            State { label: "b".into(), pattern: vec![1] },
        ];
        let mut sb = SuperBit::with_states(
            vec![0, 1],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();

        // Reward state 0
        let (old, new) = evolve_learning(&mut sb, 0, 0.3).unwrap();
        assert!((old - 0.5).abs() < 1e-10);
        assert!(new > 0.5); // weight increased
        let sum: f64 = sb.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10); // still normalized
        assert_eq!(sb.generation, 1);
    }

    #[test]
    fn test_learning_evolution_negative_reward() {
        let states = vec![
            State { label: "a".into(), pattern: vec![0] },
            State { label: "b".into(), pattern: vec![1] },
        ];
        let mut sb = SuperBit::with_states(
            vec![0, 1],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();

        // Penalize state 0
        let (_, new) = evolve_learning(&mut sb, 0, -0.3).unwrap();
        assert!(new < 0.5); // weight decreased
    }

    #[test]
    fn test_learning_evolution_clamps_negative() {
        let states = vec![
            State { label: "a".into(), pattern: vec![0] },
            State { label: "b".into(), pattern: vec![1] },
        ];
        let mut sb = SuperBit::with_states(
            vec![0, 1],
            states,
            vec![0.1, 0.9],
            DefinitionsList::new(),
        ).unwrap();

        // Heavy penalty that would make weight negative
        evolve_learning(&mut sb, 0, -1.0).unwrap();
        assert!(sb.weights[0] >= 0.0); // clamped
        assert!((sb.weights.iter().sum::<f64>() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_learning_evolution_out_of_bounds() {
        let mut sb = SuperBit::from_bits(vec![0, 1]);
        let result = evolve_learning(&mut sb, 5, 0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_evolve_n() {
        let mut sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1]);
        let results = evolve_dimensional_n(&mut sb, 10);
        assert_eq!(results.len(), 10);
        assert_eq!(sb.generation, 10);
    }

    // -------------------------------------------------------------------
    // Cascade Evolution Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_cascade_evolution_basic() {
        let mut sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1, 0, 1, 0]);
        let original = sb.sigma.clone();
        let result = evolve_cascade(&mut sb);
        assert_eq!(sb.generation, 1);
        assert!(result.modified_position.is_some());
        // Should have flipped exactly one bit
        let diff: usize = sb.sigma.iter().zip(original.iter())
            .filter(|(a, b)| a != b).count();
        assert_eq!(diff, 1);
    }

    #[test]
    fn test_cascade_evolution_not_period_2() {
        // Dimensional evolution is period-2 (same position every time).
        // Cascade evolution should NOT be — it should visit different positions.
        let mut sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1, 0, 1, 0]);
        let r1 = evolve_cascade(&mut sb);
        let r2 = evolve_cascade(&mut sb);
        let r3 = evolve_cascade(&mut sb);

        let p1 = r1.modified_position.unwrap();
        let p2 = r2.modified_position.unwrap();
        let p3 = r3.modified_position.unwrap();

        // At least two of the three positions should differ
        assert!(
            p1 != p2 || p2 != p3 || p1 != p3,
            "cascade evolution should visit different positions: {p1}, {p2}, {p3}"
        );
    }

    #[test]
    fn test_cascade_evolution_visits_all_positions() {
        // Over enough steps, every position should be visited at least once
        let n = 8;
        let mut sb = SuperBit::from_bits(vec![0; n]);
        let mut visited = std::collections::HashSet::new();

        for _ in 0..(n * 3) {
            let result = evolve_cascade(&mut sb);
            if let Some(pos) = result.modified_position {
                visited.insert(pos);
            }
        }

        // Should have visited all positions
        assert_eq!(
            visited.len(), n,
            "cascade evolution should visit all {n} positions, visited {:?}", visited
        );
    }

    #[test]
    fn test_cascade_evolution_respects_anchors() {
        let mut sb = SuperBit::from_bits(vec![1, 0, 1, 0]);
        // Anchor all positions except one
        sb.anchors.add(0);
        sb.anchors.add(1);
        sb.anchors.add(2);
        // Position 3 is free

        let mut found_blocked = false;
        let mut found_free = false;
        for _ in 0..20 {
            let result = evolve_cascade(&mut sb);
            if result.blocked_by_anchor {
                found_blocked = true;
            }
            if result.modified_position == Some(3) {
                found_free = true;
            }
        }
        assert!(found_blocked, "should encounter blocked positions");
        assert!(found_free, "should eventually hit the free position");
    }

    #[test]
    fn test_cascade_evolution_deterministic() {
        // Same starting state → same evolution sequence
        let mut sb1 = SuperBit::from_bits(vec![1, 0, 1, 1, 0]);
        let mut sb2 = SuperBit::from_bits(vec![1, 0, 1, 1, 0]);

        for _ in 0..10 {
            let r1 = evolve_cascade(&mut sb1);
            let r2 = evolve_cascade(&mut sb2);
            assert_eq!(r1.modified_position, r2.modified_position);
        }
        assert_eq!(sb1.sigma, sb2.sigma);
    }

    // -------------------------------------------------------------------
    // Non-Destructive Preview Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_evolve_dimensional_preview() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1]);
        let sigma_before = sb.sigma.clone();
        let gen_before = sb.generation;

        let (evolved, result) = evolve_dimensional_preview(&sb);

        // Original must be completely untouched
        assert_eq!(sb.sigma, sigma_before);
        assert_eq!(sb.generation, gen_before);
        // Evolved fork should show the change
        assert_eq!(evolved.generation, gen_before + 1);
        assert!(result.modified_position.is_some());
    }

    #[test]
    fn test_evolve_cascade_preview() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 1, 0, 0, 1, 0]);
        let sigma_before = sb.sigma.clone();

        let (evolved, result) = evolve_cascade_preview(&sb);

        // Original untouched
        assert_eq!(sb.sigma, sigma_before);
        assert_eq!(sb.generation, 0);
        // Fork evolved
        assert_eq!(evolved.generation, 1);
        assert!(result.modified_position.is_some());
        // They differ in exactly one position
        let diff: usize = evolved.sigma.iter().zip(sb.sigma.iter())
            .filter(|(a, b)| a != b).count();
        assert_eq!(diff, 1);
    }

    #[test]
    fn test_preview_vs_actual_match() {
        // Preview should produce identical results to actual evolution
        let sb = SuperBit::from_bits(vec![0, 1, 1, 0, 1]);

        // Preview
        let (preview, preview_result) = evolve_cascade_preview(&sb);

        // Actual
        let mut actual = sb.clone();
        let actual_result = evolve_cascade(&mut actual);

        assert_eq!(preview.sigma, actual.sigma);
        assert_eq!(preview.generation, actual.generation);
        assert_eq!(preview_result.modified_position, actual_result.modified_position);
    }
}
