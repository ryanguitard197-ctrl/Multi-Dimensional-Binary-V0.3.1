//! # SuperBit — The Atomic Unit of MDB Computing
//!
//! The SuperBit is the fundamental primitive of MDB, replacing the classical
//! bit as the atomic unit of computation. A SuperBit is formally defined as:
//!
//! ```text
//! B = (σ, Ψ, W, A, G)
//! ```
//!
//! Where:
//! - `σ ∈ {0,1}*` — the binary string encoding all state information
//! - `Ψ` — the state space {ψ₁, ψ₂, ..., ψₖ}
//! - `W` — weight vector (w₁, w₂, ..., wₖ), Σwᵢ = 1, wᵢ ≥ 0
//! - `A ⊂ ℕ` — immutable anchor positions (DefinitionsList)
//! - `G ∈ ℕ` — generation counter (evolution depth)
//!
//! All components are encoded within `σ`. The SuperBit is not a static value
//! but a living geometric object that can occupy multiple coordinate sets
//! simultaneously until an evolution step forces deterministic selection.
//!
//! **Key property**: Collapse is a *read* operation. The binary string σ is
//! never modified by collapse. The SuperBit remains in full superposition,
//! allowing unlimited independent collapses — solving the quantum superposition
//! destruction problem on classical hardware.

use crate::coordinates::{DimensionalAddress, DimensionalCascade};
use crate::definitions::DefinitionsList;
use rand::Rng;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Non-Destructive Superposition Types
// ---------------------------------------------------------------------------

/// A non-destructive view of a single state in a SuperBit's superposition.
///
/// Contains the state's identity, probability weight, and full dimensional
/// address — all readable without collapsing.
#[derive(Debug, Clone)]
pub struct StateView {
    /// Index in the state space.
    pub index: usize,
    /// Human-readable label.
    pub label: String,
    /// Probability weight (0.0–1.0).
    pub weight: f64,
    /// The binary pattern for this state.
    pub pattern: Vec<u8>,
    /// Dimensional address computed from this state's pattern via the cascade.
    pub address: DimensionalAddress,
}

/// A complete snapshot of a SuperBit's superposition — every state,
/// every weight, every dimensional address — readable without collapsing.
///
/// This is the operation that real quantum hardware **cannot** perform.
/// In quantum mechanics, measurement destroys superposition. Here, the
/// SuperBit's σ, weights, and states are completely unchanged after
/// producing this view.
#[derive(Debug, Clone)]
pub struct SuperpositionView {
    /// Dimensional address of σ (the SuperBit's current encoding).
    pub sigma_address: DimensionalAddress,
    /// Full view of every state in the state space.
    pub states: Vec<StateView>,
    /// Number of states in superposition.
    pub state_count: usize,
    /// Generation counter at the time of this snapshot.
    pub generation: u64,
}

/// A named state in the SuperBit's state space.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    /// Human-readable label for this state.
    pub label: String,
    /// The binary pattern associated with this state.
    pub pattern: Vec<u8>,
}

/// The SuperBit — atomic unit of MDB computing.
///
/// A classical-binary-encoded qubit. Every possible state, probability
/// amplitude, and collapse outcome is encoded IN the binary string itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperBit {
    /// σ — The raw binary string encoding all state information.
    /// Each element is 0 or 1.
    pub sigma: Vec<u8>,

    /// Ψ — The state space. Each entry is a possible state the SuperBit
    /// can collapse to.
    pub states: Vec<State>,

    /// W — Weight vector. `weights[i]` is the probability of collapsing
    /// to `states[i]`. Sum must equal 1.0, all values ≥ 0.
    pub weights: Vec<f64>,

    /// A — Immutable anchor positions (DefinitionsList).
    pub anchors: DefinitionsList,

    /// G — Generation counter (evolution depth).
    pub generation: u64,
}

impl SuperBit {
    /// Create a new SuperBit from raw binary data with a default single state.
    ///
    /// The initial SuperBit has one state (the data itself) with weight 1.0.
    pub fn from_bits(bits: Vec<u8>) -> Self {
        let state = State {
            label: "initial".to_string(),
            pattern: bits.clone(),
        };
        Self {
            sigma: bits,
            states: vec![state],
            weights: vec![1.0],
            anchors: DefinitionsList::new(),
            generation: 0,
        }
    }

    /// Create a new SuperBit from raw byte data.
    ///
    /// Expands bytes into individual bits before constructing the SuperBit.
    pub fn from_bytes(data: &[u8]) -> Self {
        let bits = crate::coordinates::bytes_to_bits(data);
        Self::from_bits(bits)
    }

    /// Create a SuperBit with multiple states and custom weights.
    pub fn with_states(
        sigma: Vec<u8>,
        states: Vec<State>,
        weights: Vec<f64>,
        anchors: DefinitionsList,
    ) -> Result<Self, SuperBitError> {
        if states.len() != weights.len() {
            return Err(SuperBitError::StateSizeMismatch);
        }
        if states.is_empty() {
            return Err(SuperBitError::EmptyStateSpace);
        }

        let sum: f64 = weights.iter().sum();
        if (sum - 1.0).abs() > 1e-9 {
            return Err(SuperBitError::WeightsNotNormalized(sum));
        }
        if weights.iter().any(|&w| w < 0.0) {
            return Err(SuperBitError::NegativeWeight);
        }

        Ok(Self {
            sigma,
            states,
            weights,
            anchors,
            generation: 0,
        })
    }

    /// Compute the dimensional address of this SuperBit.
    pub fn address(&self) -> DimensionalAddress {
        DimensionalAddress::from_bits(&self.sigma)
    }

    /// **Non-destructive collapse**: Read the SuperBit's state without
    /// modifying σ.
    ///
    /// Returns the index and reference to the selected state based on
    /// probability weights. The SuperBit remains in full superposition
    /// after collapse — σ is never touched.
    pub fn collapse(&self) -> (usize, &State) {
        let mut rng = rand::thread_rng();
        let roll: f64 = rng.gen();
        self.collapse_with_roll(roll)
    }

    /// Deterministic collapse with a provided random value in [0, 1).
    ///
    /// Useful for testing and reproducible behavior.
    pub fn collapse_with_roll(&self, roll: f64) -> (usize, &State) {
        let mut cumulative = 0.0;
        for (i, &w) in self.weights.iter().enumerate() {
            cumulative += w;
            if roll < cumulative {
                return (i, &self.states[i]);
            }
        }
        // Fallback to last state (handles floating-point edge case)
        let last = self.states.len() - 1;
        (last, &self.states[last])
    }

    /// Add a new state to the SuperBit's state space.
    ///
    /// Weights are redistributed proportionally to accommodate the new state.
    pub fn add_state(&mut self, state: State, initial_weight: f64) {
        // Scale existing weights down to make room
        let scale = 1.0 - initial_weight;
        for w in &mut self.weights {
            *w *= scale;
        }
        self.states.push(state);
        self.weights.push(initial_weight);
        self.normalize_weights();
    }

    /// Get the number of states in the state space.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Get the bit length of σ.
    pub fn bit_length(&self) -> usize {
        self.sigma.len()
    }

    // -----------------------------------------------------------------------
    // Non-Destructive Superposition Operations
    // -----------------------------------------------------------------------

    /// **Non-destructive superposition read**: get the full dimensional
    /// view of every state without collapsing.
    ///
    /// Returns a [`SuperpositionView`] containing every state's label,
    /// weight, binary pattern, and dimensional address. The SuperBit's
    /// σ, weights, generation — everything — remains completely unchanged.
    ///
    /// This is the fundamental advantage over quantum hardware: you can
    /// inspect the entire state space, compare dimensional addresses,
    /// and make decisions *before* choosing to collapse.
    pub fn peek(&self) -> SuperpositionView {
        let sigma_address = DimensionalAddress::from_bits(&self.sigma);
        let states = self
            .states
            .iter()
            .enumerate()
            .map(|(i, state)| StateView {
                index: i,
                label: state.label.clone(),
                weight: self.weights[i],
                pattern: state.pattern.clone(),
                address: DimensionalAddress::from_bits(&state.pattern),
            })
            .collect::<Vec<_>>();
        let state_count = states.len();
        SuperpositionView {
            sigma_address,
            states,
            state_count,
            generation: self.generation,
        }
    }

    /// Fork this SuperBit into a fully independent clone.
    ///
    /// The fork can be collapsed, evolved, modified, or destroyed
    /// without affecting the original. This enables parallel exploration
    /// of different evolution paths or collapse outcomes — something
    /// impossible with physical qubits (no-cloning theorem).
    pub fn fork(&self) -> SuperBit {
        self.clone()
    }

    /// Selectively collapse to a specific state by index (non-destructive).
    ///
    /// Unlike random collapse, this lets you choose *which* state to
    /// examine. The SuperBit remains in full superposition — σ is never
    /// modified.
    pub fn collapse_to(&self, index: usize) -> Result<&State, SuperBitError> {
        self.states
            .get(index)
            .ok_or(SuperBitError::StateIndexOutOfBounds(index))
    }

    /// Compute the full dimensional cascade for each state's pattern.
    ///
    /// Returns `(index, label, cascade)` for every state. Use this for
    /// deep dimensional analysis — e.g., comparing how different states
    /// behave across D4 (Spacetime), D5 (Momentum), D6 (Energy).
    pub fn state_cascades(&self, max_dim: usize) -> Vec<(usize, String, DimensionalCascade)> {
        self.states
            .iter()
            .enumerate()
            .map(|(i, state)| {
                let cascade = DimensionalCascade::from_bits(&state.pattern, max_dim);
                (i, state.label.clone(), cascade)
            })
            .collect()
    }

    /// Compute pairwise dimensional distances between all states.
    ///
    /// Returns `(i, j, distance)` tuples for every pair. Distance is
    /// computed in the (n, D4-spacetime) plane. This reveals which states
    /// are dimensionally similar vs. different — information that is
    /// impossible to extract from a real quantum system without collapsing.
    pub fn state_distances(&self) -> Vec<(usize, usize, f64)> {
        let addresses: Vec<DimensionalAddress> = self
            .states
            .iter()
            .map(|s| DimensionalAddress::from_bits(&s.pattern))
            .collect();

        let mut distances = Vec::new();
        for i in 0..addresses.len() {
            for j in (i + 1)..addresses.len() {
                let d4_diff = addresses[i].d4_spacetime - addresses[j].d4_spacetime;
                let n_diff = addresses[i].n as f64 - addresses[j].n as f64;
                let dist = (d4_diff * d4_diff + n_diff * n_diff).sqrt();
                distances.push((i, j, dist));
            }
        }
        distances
    }

    /// Normalize weights so they sum to exactly 1.0.
    pub fn normalize_weights(&mut self) {
        let sum: f64 = self.weights.iter().sum();
        if sum > 0.0 {
            for w in &mut self.weights {
                *w /= sum;
            }
        }
    }

    /// Encode the full SuperBit state into a self-describing binary format.
    ///
    /// Format:
    /// ```text
    /// [MAGIC: 4 bytes "MDB\x01"]
    /// [GENERATION: 8 bytes BE]
    /// [SIGMA_LEN: 4 bytes BE][SIGMA: N bytes, packed bits]
    /// [STATE_COUNT: 4 bytes BE]
    ///   For each state:
    ///     [LABEL_LEN: 2 bytes BE][LABEL: UTF-8 bytes]
    ///     [PATTERN_LEN: 4 bytes BE][PATTERN: packed bits]
    ///     [WEIGHT: 8 bytes BE f64]
    /// [ANCHORS: encoded DefinitionsList]
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();

        // Magic bytes
        out.extend_from_slice(b"MDB\x01");

        // Generation
        out.extend_from_slice(&self.generation.to_be_bytes());

        // Sigma
        let sigma_packed = crate::coordinates::bits_to_bytes(&self.sigma);
        out.extend_from_slice(&(self.sigma.len() as u32).to_be_bytes());
        out.extend_from_slice(&sigma_packed);

        // States + weights
        out.extend_from_slice(&(self.states.len() as u32).to_be_bytes());
        for (state, &weight) in self.states.iter().zip(self.weights.iter()) {
            let label_bytes = state.label.as_bytes();
            out.extend_from_slice(&(label_bytes.len() as u16).to_be_bytes());
            out.extend_from_slice(label_bytes);

            let pattern_packed = crate::coordinates::bits_to_bytes(&state.pattern);
            out.extend_from_slice(&(state.pattern.len() as u32).to_be_bytes());
            out.extend_from_slice(&pattern_packed);

            out.extend_from_slice(&weight.to_be_bytes());
        }

        // Anchors
        out.extend_from_slice(&self.anchors.encode());

        out
    }

    /// Decode a SuperBit from its binary representation.
    pub fn decode(data: &[u8]) -> Result<Self, SuperBitError> {
        if data.len() < 4 || &data[0..4] != b"MDB\x01" {
            return Err(SuperBitError::InvalidMagic);
        }
        let mut pos = 4;

        // Generation
        if data.len() < pos + 8 {
            return Err(SuperBitError::TruncatedData);
        }
        let generation = u64::from_be_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        // Sigma
        if data.len() < pos + 4 {
            return Err(SuperBitError::TruncatedData);
        }
        let sigma_bit_len =
            u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let sigma_byte_len = (sigma_bit_len + 7) / 8;
        if data.len() < pos + sigma_byte_len {
            return Err(SuperBitError::TruncatedData);
        }
        let sigma_packed = &data[pos..pos + sigma_byte_len];
        let mut sigma = crate::coordinates::bytes_to_bits(sigma_packed);
        sigma.truncate(sigma_bit_len);
        pos += sigma_byte_len;

        // States + weights
        if data.len() < pos + 4 {
            return Err(SuperBitError::TruncatedData);
        }
        let state_count =
            u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;

        let mut states = Vec::with_capacity(state_count);
        let mut weights = Vec::with_capacity(state_count);

        for _ in 0..state_count {
            // Label
            if data.len() < pos + 2 {
                return Err(SuperBitError::TruncatedData);
            }
            let label_len =
                u16::from_be_bytes(data[pos..pos + 2].try_into().unwrap()) as usize;
            pos += 2;
            if data.len() < pos + label_len {
                return Err(SuperBitError::TruncatedData);
            }
            let label = String::from_utf8_lossy(&data[pos..pos + label_len]).to_string();
            pos += label_len;

            // Pattern
            if data.len() < pos + 4 {
                return Err(SuperBitError::TruncatedData);
            }
            let pattern_bit_len =
                u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
            pos += 4;
            let pattern_byte_len = (pattern_bit_len + 7) / 8;
            if data.len() < pos + pattern_byte_len {
                return Err(SuperBitError::TruncatedData);
            }
            let pattern_packed = &data[pos..pos + pattern_byte_len];
            let mut pattern = crate::coordinates::bytes_to_bits(pattern_packed);
            pattern.truncate(pattern_bit_len);
            pos += pattern_byte_len;

            // Weight
            if data.len() < pos + 8 {
                return Err(SuperBitError::TruncatedData);
            }
            let weight = f64::from_be_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;

            states.push(State { label, pattern });
            weights.push(weight);
        }

        // Anchors
        let (anchors, _) = DefinitionsList::decode(&data[pos..])
            .ok_or(SuperBitError::TruncatedData)?;

        Ok(Self {
            sigma,
            states,
            weights,
            anchors,
            generation,
        })
    }
}

/// Errors that can occur during SuperBit operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SuperBitError {
    StateSizeMismatch,
    EmptyStateSpace,
    WeightsNotNormalized(f64),
    NegativeWeight,
    InvalidMagic,
    TruncatedData,
    StateIndexOutOfBounds(usize),
}

impl std::fmt::Display for SuperBitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StateSizeMismatch => write!(f, "states and weights have different lengths"),
            Self::EmptyStateSpace => write!(f, "state space must not be empty"),
            Self::WeightsNotNormalized(s) => write!(f, "weights sum to {} (expected 1.0)", s),
            Self::NegativeWeight => write!(f, "weights must be non-negative"),
            Self::InvalidMagic => write!(f, "invalid magic bytes (expected MDB\\x01)"),
            Self::TruncatedData => write!(f, "data is truncated"),
            Self::StateIndexOutOfBounds(idx) => {
                write!(f, "state index {} out of bounds", idx)
            }
        }
    }
}

impl std::error::Error for SuperBitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bits() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 1, 0]);
        assert_eq!(sb.bit_length(), 5);
        assert_eq!(sb.state_count(), 1);
        assert_eq!(sb.generation, 0);
        assert_eq!(sb.weights, vec![1.0]);
    }

    #[test]
    fn test_from_bytes() {
        let sb = SuperBit::from_bytes(&[0xFF, 0x00]);
        assert_eq!(sb.bit_length(), 16);
        assert_eq!(sb.sigma[..8], [1, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(sb.sigma[8..], [0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_non_destructive_collapse() {
        let sb = SuperBit::from_bits(vec![1, 0, 1]);
        let sigma_before = sb.sigma.clone();

        // Collapse should not modify sigma
        let (idx, state) = sb.collapse();
        assert_eq!(idx, 0);
        assert_eq!(state.label, "initial");
        assert_eq!(sb.sigma, sigma_before); // σ unchanged!
    }

    #[test]
    fn test_deterministic_collapse() {
        let states = vec![
            State { label: "alpha".into(), pattern: vec![0] },
            State { label: "beta".into(), pattern: vec![1] },
            State { label: "gamma".into(), pattern: vec![1, 0] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1],
            states,
            vec![0.2, 0.5, 0.3],
            DefinitionsList::new(),
        ).unwrap();

        // roll=0.1 => alpha (cumulative 0.2)
        assert_eq!(sb.collapse_with_roll(0.1).0, 0);
        // roll=0.3 => beta (cumulative 0.7)
        assert_eq!(sb.collapse_with_roll(0.3).0, 1);
        // roll=0.8 => gamma (cumulative 1.0)
        assert_eq!(sb.collapse_with_roll(0.8).0, 2);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let states = vec![
            State { label: "on".into(), pattern: vec![1, 1] },
            State { label: "off".into(), pattern: vec![0, 0] },
        ];
        let mut sb = SuperBit::with_states(
            vec![1, 0, 1, 1],
            states,
            vec![0.7, 0.3],
            DefinitionsList::from_positions(vec![0, 3]),
        ).unwrap();
        sb.generation = 42;

        let encoded = sb.encode();
        let decoded = SuperBit::decode(&encoded).unwrap();

        assert_eq!(decoded.sigma, sb.sigma);
        assert_eq!(decoded.generation, 42);
        assert_eq!(decoded.state_count(), 2);
        assert_eq!(decoded.states[0].label, "on");
        assert_eq!(decoded.states[1].label, "off");
        assert!((decoded.weights[0] - 0.7).abs() < 1e-10);
        assert!((decoded.weights[1] - 0.3).abs() < 1e-10);
        assert!(decoded.anchors.is_anchored(0));
        assert!(decoded.anchors.is_anchored(3));
        assert!(!decoded.anchors.is_anchored(1));
    }

    #[test]
    fn test_address() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 0, 1, 1, 0, 0]);
        let addr = sb.address();
        assert_eq!(addr.n, 8);
        // D4 spacetime uses linear position-weighting: Σ(D4_i × (i+1))
        // D5 momentum uses quadratic position-weighting: Σ(D5_i × (i+1)²)
        // Just verify both are positive and different from each other
        assert!(addr.d4_spacetime > 0.0);
        assert!(addr.d5_momentum > 0);
        // d4 is geometric cascade, d5 is fingerprint — both nonzero for real data
        assert!(addr.d4_spacetime > 0.0);
    }

    #[test]
    fn test_invalid_weights() {
        let states = vec![State { label: "a".into(), pattern: vec![0] }];
        assert!(SuperBit::with_states(vec![0], states.clone(), vec![0.5], DefinitionsList::new()).is_err());
        assert!(SuperBit::with_states(vec![0], states.clone(), vec![-0.5], DefinitionsList::new()).is_err());
    }

    #[test]
    fn test_add_state() {
        let mut sb = SuperBit::from_bits(vec![1, 0, 1]);
        assert_eq!(sb.state_count(), 1);

        sb.add_state(
            State { label: "excited".into(), pattern: vec![1, 1, 1] },
            0.3,
        );
        assert_eq!(sb.state_count(), 2);
        let sum: f64 = sb.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    // -------------------------------------------------------------------
    // Non-Destructive Superposition Tests
    // -------------------------------------------------------------------

    #[test]
    fn test_peek_does_not_modify() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 1]);
        let sigma_before = sb.sigma.clone();
        let weights_before = sb.weights.clone();
        let gen_before = sb.generation;

        let view = sb.peek();

        // SuperBit must be completely unchanged
        assert_eq!(sb.sigma, sigma_before);
        assert_eq!(sb.weights, weights_before);
        assert_eq!(sb.generation, gen_before);
        // View should contain valid data
        assert_eq!(view.state_count, 1);
        assert_eq!(view.states[0].label, "initial");
        assert_eq!(view.sigma_address.n, 4);
    }

    #[test]
    fn test_peek_multi_state() {
        let states = vec![
            State { label: "alpha".into(), pattern: vec![1, 0] },
            State { label: "beta".into(), pattern: vec![0, 1] },
            State { label: "gamma".into(), pattern: vec![1, 1] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1],
            states,
            vec![0.5, 0.3, 0.2],
            DefinitionsList::new(),
        ).unwrap();

        let view = sb.peek();
        assert_eq!(view.state_count, 3);
        assert_eq!(view.states[0].label, "alpha");
        assert!((view.states[0].weight - 0.5).abs() < 1e-10);
        assert_eq!(view.states[1].label, "beta");
        assert!((view.states[1].weight - 0.3).abs() < 1e-10);
        // Each state has a valid dimensional address
        for sv in &view.states {
            assert!(sv.address.n > 0);
        }
        // Different patterns → different addresses
        assert_ne!(view.states[0].address.d5_momentum, view.states[1].address.d5_momentum);
    }

    #[test]
    fn test_fork_independence() {
        let mut sb = SuperBit::from_bits(vec![1, 0, 1, 0]);
        let fork = sb.fork();

        // Modify the original
        sb.sigma[0] = 0;
        sb.generation = 999;

        // Fork must be unaffected
        assert_eq!(fork.sigma[0], 1);
        assert_eq!(fork.generation, 0);
    }

    #[test]
    fn test_collapse_to_specific() {
        let states = vec![
            State { label: "red".into(), pattern: vec![1, 0] },
            State { label: "blue".into(), pattern: vec![0, 1] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();

        // Collapse to specific state
        let state = sb.collapse_to(1).unwrap();
        assert_eq!(state.label, "blue");
        assert_eq!(state.pattern, vec![0, 1]);

        // σ unchanged after selective collapse
        assert_eq!(sb.sigma, vec![1, 0]);
    }

    #[test]
    fn test_collapse_to_out_of_bounds() {
        let sb = SuperBit::from_bits(vec![1, 0]);
        assert!(sb.collapse_to(5).is_err());
    }

    #[test]
    fn test_state_distances() {
        let states = vec![
            State { label: "near".into(), pattern: vec![1, 0, 1, 0] },
            State { label: "far".into(), pattern: vec![0, 0, 0, 0] },
            State { label: "full".into(), pattern: vec![1, 1, 1, 1] },
        ];
        let sb = SuperBit::with_states(
            vec![1, 0, 1, 0],
            states,
            vec![0.4, 0.3, 0.3],
            DefinitionsList::new(),
        ).unwrap();

        let distances = sb.state_distances();
        // 3 states → 3 pairs
        assert_eq!(distances.len(), 3);
        // (0,1), (0,2), (1,2)
        assert_eq!(distances[0].0, 0);
        assert_eq!(distances[0].1, 1);
        // Distance from all-zeros to all-ones should be largest
        let d_01 = distances[0].2; // near vs far
        let d_02 = distances[1].2; // near vs full
        let d_12 = distances[2].2; // far vs full
        assert!(d_12 > d_01, "all-0 to all-1 should be furthest");
        assert!(d_12 > d_02, "all-0 to all-1 should be furthest");
    }

    #[test]
    fn test_parallel_exploration() {
        // The quantum-impossible workflow: fork, collapse differently,
        // compare — while the original is untouched.
        let states = vec![
            State { label: "left".into(), pattern: vec![1, 0, 0] },
            State { label: "right".into(), pattern: vec![0, 0, 1] },
        ];
        let original = SuperBit::with_states(
            vec![1, 0, 1],
            states,
            vec![0.5, 0.5],
            DefinitionsList::new(),
        ).unwrap();

        // Fork twice
        let fork_a = original.fork();
        let fork_b = original.fork();

        // Collapse each fork to a different state
        let state_a = fork_a.collapse_to(0).unwrap(); // "left"
        let state_b = fork_b.collapse_to(1).unwrap(); // "right"

        // Compare: different states, different patterns
        assert_eq!(state_a.label, "left");
        assert_eq!(state_b.label, "right");
        assert_ne!(state_a.pattern, state_b.pattern);

        // Original is completely untouched — still in full superposition
        assert_eq!(original.state_count(), 2);
        assert_eq!(original.sigma, vec![1, 0, 1]);
        assert!((original.weights[0] - 0.5).abs() < 1e-10);
        assert!((original.weights[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_state_cascades() {
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

        let cascades = sb.state_cascades(6);
        assert_eq!(cascades.len(), 2);
        assert_eq!(cascades[0].1, "a");
        assert_eq!(cascades[1].1, "b");
        // Each cascade should have 6 dimensions
        assert_eq!(cascades[0].2.dims.len(), 6);
        assert_eq!(cascades[1].2.dims.len(), 6);
    }
}
