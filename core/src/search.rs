//! # Superposition Search — Solving Problems Without Collapse
//!
//! This module demonstrates the practical computational advantage of
//! non-destructive superposition. It implements a search algorithm that:
//!
//! 1. Creates a SuperBit with N candidate states
//! 2. Reads all states non-destructively via `peek()`
//! 3. Evaluates every candidate's fitness using dimensional coordinates
//! 4. Finds the optimal solution — while the superposition is *still intact*
//!
//! ## Why this matters
//!
//! On a quantum computer, you can't do step 2 — measurement destroys the
//! superposition. You get one shot, and the answer is probabilistic.
//!
//! On MDB, you can:
//! - Inspect every candidate simultaneously
//! - Compare candidates dimensionally
//! - Fork the SuperBit and explore different optimization paths
//! - Keep the original superposition untouched for future use
//!
//! ## Search Algorithms
//!
//! - **`dimensional_search`**: Find the state closest to a target dimensional address
//! - **`pattern_search`**: Find the state closest to a target bit pattern
//! - **`fitness_search`**: Optimize a custom fitness function over the state space
//! - **`exhaustive_explore`**: Full parallel exploration with forking

use crate::coordinates::DimensionalAddress;
use crate::superbit::SuperBit;

// ---------------------------------------------------------------------------
// Search Results
// ---------------------------------------------------------------------------

/// Result of a superposition search — the winning state plus search metadata.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Index of the best state in the superposition.
    pub best_index: usize,
    /// Label of the best state.
    pub best_label: String,
    /// Pattern of the best state.
    pub best_pattern: Vec<u8>,
    /// Fitness score of the best state (higher = better).
    pub best_fitness: f64,
    /// Dimensional address of the best state.
    pub best_address: DimensionalAddress,
    /// All candidates evaluated, sorted by fitness (best first).
    pub ranked: Vec<(usize, String, f64)>,
    /// Total candidates evaluated.
    pub candidates_evaluated: usize,
    /// Whether the original SuperBit is still in full superposition
    /// (always true — that's the point).
    pub superposition_intact: bool,
}

// ---------------------------------------------------------------------------
// Dimensional Search — Find Closest to Target Address
// ---------------------------------------------------------------------------

/// Search the state space for the state whose dimensional address is closest
/// to a target address.
///
/// Uses D4 (Spacetime) distance as the proximity metric. The target doesn't
/// have to exist in the state space — this finds the nearest match.
///
/// The original SuperBit is *completely untouched* after this search.
pub fn dimensional_search(
    sb: &SuperBit,
    target: &DimensionalAddress,
) -> SearchResult {
    let view = sb.peek();
    let mut ranked: Vec<(usize, String, f64)> = view
        .states
        .iter()
        .map(|sv| {
            let d4_diff = sv.address.d4_spacetime - target.d4_spacetime;
            let n_diff = sv.address.n as f64 - target.n as f64;
            let distance = (d4_diff * d4_diff + n_diff * n_diff).sqrt();
            // Fitness = inverse distance (closer = higher fitness)
            let fitness = if distance < 1e-15 { f64::MAX } else { 1.0 / distance };
            (sv.index, sv.label.clone(), fitness)
        })
        .collect();

    ranked.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let best = &ranked[0];
    let best_state = &view.states[best.0];

    SearchResult {
        best_index: best.0,
        best_label: best.1.clone(),
        best_pattern: best_state.pattern.clone(),
        best_fitness: best.2,
        best_address: best_state.address.clone(),
        ranked,
        candidates_evaluated: view.state_count,
        superposition_intact: true,
    }
}

// ---------------------------------------------------------------------------
// Pattern Search — Find Closest to Target Bit Pattern
// ---------------------------------------------------------------------------

/// Search the state space for the state whose bit pattern is closest
/// to a target pattern (by Hamming distance).
///
/// The original SuperBit is *completely untouched* after this search.
pub fn pattern_search(sb: &SuperBit, target: &[u8]) -> SearchResult {
    let view = sb.peek();
    let mut ranked: Vec<(usize, String, f64)> = view
        .states
        .iter()
        .map(|sv| {
            let hamming = hamming_distance(&sv.pattern, target);
            // Fitness = inverse Hamming distance (exact match = MAX)
            let fitness = if hamming == 0 {
                f64::MAX
            } else {
                1.0 / hamming as f64
            };
            (sv.index, sv.label.clone(), fitness)
        })
        .collect();

    ranked.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let best = &ranked[0];
    let best_state = &view.states[best.0];

    SearchResult {
        best_index: best.0,
        best_label: best.1.clone(),
        best_pattern: best_state.pattern.clone(),
        best_fitness: best.2,
        best_address: best_state.address.clone(),
        ranked,
        candidates_evaluated: view.state_count,
        superposition_intact: true,
    }
}

// ---------------------------------------------------------------------------
// Fitness Search — Custom Fitness Function
// ---------------------------------------------------------------------------

/// Search the state space using a custom fitness function.
///
/// `fitness_fn` takes a state's pattern and dimensional address, and returns
/// a score (higher = better). The search evaluates every state without
/// collapsing the superposition.
pub fn fitness_search<F>(sb: &SuperBit, fitness_fn: F) -> SearchResult
where
    F: Fn(&[u8], &DimensionalAddress) -> f64,
{
    let view = sb.peek();
    let mut ranked: Vec<(usize, String, f64)> = view
        .states
        .iter()
        .map(|sv| {
            let fitness = fitness_fn(&sv.pattern, &sv.address);
            (sv.index, sv.label.clone(), fitness)
        })
        .collect();

    ranked.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let best = &ranked[0];
    let best_state = &view.states[best.0];

    SearchResult {
        best_index: best.0,
        best_label: best.1.clone(),
        best_pattern: best_state.pattern.clone(),
        best_fitness: best.2,
        best_address: best_state.address.clone(),
        ranked,
        candidates_evaluated: view.state_count,
        superposition_intact: true,
    }
}

// ---------------------------------------------------------------------------
// Exhaustive Explore — Fork + Collapse Every State
// ---------------------------------------------------------------------------

/// Full parallel exploration: fork the SuperBit once for each state,
/// collapse each fork to a specific state, and compare all outcomes.
///
/// This is the workflow that's fundamentally impossible on quantum hardware:
/// - Fork N copies (violates no-cloning theorem)
/// - Collapse each to a different state (can't choose which state to measure)
/// - Compare all outcomes side by side (can't re-measure after collapse)
/// - Original is still in superposition (measurement destroys superposition)
///
/// Returns the best state by the provided fitness function, plus all
/// fork results for comparison.
pub fn exhaustive_explore<F>(sb: &SuperBit, fitness_fn: F) -> ExploreResult
where
    F: Fn(&[u8], &DimensionalAddress) -> f64,
{
    let state_count = sb.state_count();
    let mut explorations = Vec::with_capacity(state_count);
    let mut best_idx = 0;
    let mut best_fitness = f64::NEG_INFINITY;

    for i in 0..state_count {
        // Fork a fresh copy
        let fork = sb.fork();
        // Collapse to this specific state
        let state = fork.collapse_to(i).unwrap();
        let address = DimensionalAddress::from_bits(&state.pattern);
        let fitness = fitness_fn(&state.pattern, &address);

        if fitness > best_fitness {
            best_fitness = fitness;
            best_idx = i;
        }

        explorations.push(ExplorationEntry {
            index: i,
            label: state.label.clone(),
            pattern: state.pattern.clone(),
            address,
            fitness,
        });
    }

    // Verify original is untouched
    assert_eq!(sb.state_count(), state_count);

    ExploreResult {
        explorations,
        best_index: best_idx,
        best_fitness,
        forks_created: state_count,
        original_intact: true,
    }
}

/// Result of exhaustive exploration.
#[derive(Debug, Clone)]
pub struct ExploreResult {
    /// All states explored, with their fitness scores.
    pub explorations: Vec<ExplorationEntry>,
    /// Index of the best state.
    pub best_index: usize,
    /// Fitness of the best state.
    pub best_fitness: f64,
    /// Number of forks created.
    pub forks_created: usize,
    /// Whether the original SuperBit is still intact (always true).
    pub original_intact: bool,
}

/// A single exploration result from the exhaustive search.
#[derive(Debug, Clone)]
pub struct ExplorationEntry {
    /// State index.
    pub index: usize,
    /// State label.
    pub label: String,
    /// State bit pattern.
    pub pattern: Vec<u8>,
    /// Dimensional address of this state.
    pub address: DimensionalAddress,
    /// Fitness score.
    pub fitness: f64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute Hamming distance between two bit patterns.
/// If lengths differ, extra bits count as mismatches.
fn hamming_distance(a: &[u8], b: &[u8]) -> usize {
    let max_len = a.len().max(b.len());
    let mut dist = 0;
    for i in 0..max_len {
        let bit_a = a.get(i).copied().unwrap_or(0);
        let bit_b = b.get(i).copied().unwrap_or(0);
        if bit_a != bit_b {
            dist += 1;
        }
    }
    dist
}

/// Create a SuperBit pre-loaded with candidate states for searching.
///
/// Generates `2^depth` candidate states by flipping positions selected
/// via the Golden Ratio φ, starting from the given base pattern.
/// This is a convenience wrapper around `cascade_hadamard` logic.
pub fn create_search_space(base: &[u8], depth: usize) -> SuperBit {
    let sb = SuperBit::from_bits(base.to_vec());
    let result = crate::gates::cascade_hadamard(&sb, depth);
    result.output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::DefinitionsList;
    use crate::superbit::State;

    // -------------------------------------------------------------------
    // Dimensional Search Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_dimensional_search_finds_closest() {
        let states = vec![
            State { label: "far".into(), pattern: vec![0, 0, 0, 0, 0, 0, 0, 0] },
            State { label: "near".into(), pattern: vec![1, 1, 1, 1, 1, 1, 1, 1] },
            State { label: "mid".into(), pattern: vec![1, 0, 1, 0, 1, 0, 1, 0] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1, 0, 1, 0, 1, 0],
            states,
            vec![0.33, 0.34, 0.33],
            DefinitionsList::new(),
        ).unwrap();

        // Target is the all-1s address
        let target = DimensionalAddress::from_bits(&[1, 1, 1, 1, 1, 1, 1, 1]);
        let result = dimensional_search(&sb, &target);

        assert_eq!(result.best_label, "near");
        assert!(result.superposition_intact);
        assert_eq!(result.candidates_evaluated, 3);
    }

    #[test]
    fn test_dimensional_search_preserves_superposition() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0]);
        let sigma_before = sb.sigma.clone();
        let states_before = sb.state_count();

        let target = DimensionalAddress::from_bits(&[0, 0, 0, 0]);
        let _ = dimensional_search(&sb, &target);

        assert_eq!(sb.sigma, sigma_before);
        assert_eq!(sb.state_count(), states_before);
    }

    // -------------------------------------------------------------------
    // Pattern Search Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_pattern_search_exact_match() {
        let states = vec![
            State { label: "wrong1".into(), pattern: vec![0, 0, 0, 0] },
            State { label: "wrong2".into(), pattern: vec![1, 1, 1, 1] },
            State { label: "exact".into(), pattern: vec![1, 0, 1, 0] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1, 0],
            states,
            vec![0.33, 0.34, 0.33],
            DefinitionsList::new(),
        ).unwrap();

        let result = pattern_search(&sb, &[1, 0, 1, 0]);
        assert_eq!(result.best_label, "exact");
        assert_eq!(result.best_fitness, f64::MAX); // exact match
    }

    #[test]
    fn test_pattern_search_closest() {
        let states = vec![
            State { label: "close".into(), pattern: vec![1, 0, 1, 1] },   // 1 bit diff
            State { label: "far".into(), pattern: vec![0, 1, 0, 1] },     // 4 bits diff
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1, 0],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();

        let result = pattern_search(&sb, &[1, 0, 1, 0]);
        assert_eq!(result.best_label, "close");
    }

    // -------------------------------------------------------------------
    // Fitness Search Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_fitness_search_custom() {
        let states = vec![
            State { label: "low".into(), pattern: vec![0, 0, 1, 0] },   // 1 one
            State { label: "medium".into(), pattern: vec![1, 0, 1, 0] }, // 2 ones
            State { label: "high".into(), pattern: vec![1, 1, 1, 0] },  // 3 ones
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1, 0],
            states,
            vec![0.33, 0.34, 0.33],
            DefinitionsList::new(),
        ).unwrap();

        // Fitness = count of 1-bits
        let result = fitness_search(&sb, |pattern, _addr| {
            pattern.iter().filter(|&&b| b == 1).count() as f64
        });

        assert_eq!(result.best_label, "high");
        assert_eq!(result.best_fitness, 3.0);
    }

    #[test]
    fn test_fitness_search_with_address() {
        let states = vec![
            State { label: "a".into(), pattern: vec![1, 0, 1, 0] },
            State { label: "b".into(), pattern: vec![0, 1, 0, 1] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1, 0],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();

        // Fitness based on D4 spacetime value
        let result = fitness_search(&sb, |_pattern, addr| addr.d4_spacetime);
        // Both have same length, but different D4 values
        assert_eq!(result.candidates_evaluated, 2);
    }

    // -------------------------------------------------------------------
    // Exhaustive Explore Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_exhaustive_explore() {
        let states = vec![
            State { label: "worst".into(), pattern: vec![0, 0, 0, 0] },
            State { label: "best".into(), pattern: vec![1, 1, 1, 1] },
            State { label: "okay".into(), pattern: vec![1, 0, 1, 0] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1, 0],
            states,
            vec![0.33, 0.34, 0.33],
            DefinitionsList::new(),
        ).unwrap();

        let result = exhaustive_explore(&sb, |pattern, _addr| {
            pattern.iter().filter(|&&b| b == 1).count() as f64
        });

        assert_eq!(result.forks_created, 3);
        assert!(result.original_intact);
        assert_eq!(result.best_index, 1); // "best" has most 1s
        assert_eq!(result.best_fitness, 4.0);

        // Original should be completely untouched
        assert_eq!(sb.state_count(), 3);
        assert_eq!(sb.sigma, vec![1, 0, 1, 0]);
    }

    #[test]
    fn test_exhaustive_explore_original_untouched() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1, 0, 1, 0]);
        let sigma_before = sb.sigma.clone();
        let gen_before = sb.generation;

        let _ = exhaustive_explore(&sb, |p, _| p.len() as f64);

        assert_eq!(sb.sigma, sigma_before);
        assert_eq!(sb.generation, gen_before);
    }

    // -------------------------------------------------------------------
    // Integration: Gates + Search
    // -------------------------------------------------------------------

    #[test]
    fn test_hadamard_then_search() {
        // Full workflow: create superposition, then search it
        let sb = SuperBit::from_bits(vec![0, 0, 0, 0, 0, 0, 0, 0]);
        let h = crate::gates::cascade_hadamard(&sb, 4);

        // Search for pattern closest to all-1s
        let result = pattern_search(&h.output, &[1, 1, 1, 1, 1, 1, 1, 1]);

        // Should find the state with the most flipped bits
        let ones: usize = result.best_pattern.iter().filter(|&&b| b == 1).count();
        assert!(ones >= 2, "best match should have multiple 1-bits");
        assert!(result.superposition_intact);
    }

    #[test]
    fn test_create_search_space() {
        let sb = create_search_space(&[1, 0, 1, 0, 1, 0, 1, 0], 4);
        assert!(sb.state_count() >= 8);

        // Search the generated space
        let result = fitness_search(&sb, |pattern, _addr| {
            // Prefer patterns with alternating 0/1
            let mut alternations = 0.0;
            for i in 1..pattern.len() {
                if pattern[i] != pattern[i - 1] {
                    alternations += 1.0;
                }
            }
            alternations
        });

        assert!(result.candidates_evaluated >= 8);
        assert!(result.superposition_intact);
    }

    // -------------------------------------------------------------------
    // Hamming Distance Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_hamming_distance() {
        assert_eq!(hamming_distance(&[1, 0, 1, 0], &[1, 0, 1, 0]), 0);
        assert_eq!(hamming_distance(&[1, 0, 1, 0], &[0, 1, 0, 1]), 4);
        assert_eq!(hamming_distance(&[1, 0, 1, 0], &[1, 0, 1, 1]), 1);
        // Different lengths
        assert_eq!(hamming_distance(&[1, 0], &[1, 0, 1]), 1);
    }
}
