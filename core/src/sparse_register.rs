//! # Sparse Quantum Register — Cascade-Addressed
//!
//! A quantum register that stores only basis states with non-negligible
//! amplitude, indexed by their MDB dimensional cascade coordinates.
//!
//! ## Why This Exists
//!
//! The dense register (`register.rs`) allocates 2^n complex amplitudes
//! up front.  At 24 qubits that's 256MB; at 30 it's 16GB; at 40 it's
//! 16TB.  This exponential wall is inherent to the dense representation —
//! every possible basis state gets an entry whether it has amplitude or not.
//!
//! The sparse register breaks this coupling.  Memory scales with the
//! number of *populated* basis states — the actual entanglement complexity
//! of the circuit — not with 2^n.  For circuits that stay sparse throughout
//! (Grover's, shallow VQE, QAOA, many error-correction rounds), this
//! enables scaling well past 24 qubits on commodity hardware.
//!
//! ## How It Works
//!
//! Each populated basis state |k⟩ is stored in a `HashMap<u64, C>` keyed
//! by the integer basis index `k`.  The cascade coordinates are computed
//! on demand via `DimensionalAddress::from_bits()` for any operation that
//! needs them (persistence, cross-register comparison, export).
//!
//! Gate operations iterate only over populated entries:
//! - **Single-qubit gates**: for each (index, amp), compute partner index
//!   by flipping the target bit, apply the 2×2 unitary.
//! - **Controlled gates**: for each (index, amp) where control bit is set,
//!   compute target partner, apply unitary.
//! - **Multi-controlled gates**: same pattern, check all control bits.
//!
//! After any gate, new basis states may be populated (entanglement growth).
//! After measurement or pruning, states may be removed (sparsification).
//!
//! ## Pruning
//!
//! `prune(threshold)` removes all basis states with |amplitude|² below
//! `threshold`, then renormalises.  This is the key to staying sparse:
//! after each layer of gates, prune negligible tails.  The threshold is
//! configurable; `1e-15` is lossless cleanup, `1e-10` is safe for most
//! algorithms, `1e-6` trades precision for speed at high qubit counts.
//!
//! ## Cascade Integration
//!
//! The cascade address for any basis state can be computed in O(n) from its
//! bit representation.  The `cascade_snapshot()` method returns every
//! populated state's full `DimensionalAddress` — useful for dimensional
//! analysis, persistence, and cross-register operations.
//!
//! ## Non-Destructive Operations (MDB Exclusive)
//!
//! Like the dense register, the sparse register supports `peek()`, `fork()`,
//! and `sample_all()` — reading state without collapse.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt;

use crate::coordinates::DimensionalAddress;
use crate::register::{FullMeasurement, PositionMeasurement, RegisterView, RegisterStateView};

// ── Complex number helpers ──────────────────────────────────────────

type C = (f64, f64);

const C_ZERO: C = (0.0, 0.0);
const C_ONE: C = (1.0, 0.0);
const SQRT2_INV: f64 = std::f64::consts::FRAC_1_SQRT_2;

#[inline]
fn c_add(a: C, b: C) -> C {
    (a.0 + b.0, a.1 + b.1)
}

#[inline]
fn c_mul(a: C, b: C) -> C {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline]
fn c_scale(s: f64, a: C) -> C {
    (s * a.0, s * a.1)
}

#[inline]
fn c_norm_sq(a: C) -> f64 {
    a.0 * a.0 + a.1 * a.1
}

#[inline]
fn c_phase(theta: f64) -> C {
    (theta.cos(), theta.sin())
}

// ── PRNG ────────────────────────────────────────────────────────────

fn splitmix64(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

// ── Bit helpers ─────────────────────────────────────────────────────

fn index_to_bits(index: u64, n: usize) -> Vec<u8> {
    (0..n)
        .map(|k| ((index >> (n - 1 - k)) & 1) as u8)
        .collect()
}

fn bits_to_index(bits: &[u8]) -> u64 {
    bits.iter().fold(0u64, |acc, &b| (acc << 1) | (b as u64))
}

/// Extract a contiguous field of `len` bits starting at position `start` (MSB-first).
fn extract_field(index: u64, start: usize, len: usize, n: usize) -> u64 {
    let mut val = 0u64;
    for k in 0..len {
        let bit_pos = n - 1 - (start + k);
        val = (val << 1) | ((index >> bit_pos) & 1);
    }
    val
}

/// Replace a contiguous field of `len` bits starting at position `start`.
fn replace_field(index: u64, start: usize, len: usize, value: u64, n: usize) -> u64 {
    let mut result = index;
    for k in 0..len {
        let bit_pos = n - 1 - (start + k);
        let bit_val = (value >> (len - 1 - k)) & 1;
        if bit_val == 1 {
            result |= 1 << bit_pos;
        } else {
            result &= !(1u64 << bit_pos);
        }
    }
    result
}

// ── Cascade snapshot entry ──────────────────────────────────────────

/// A populated basis state with its cascade address and amplitude.
pub struct CascadeEntry {
    pub index: u64,
    pub bits: Vec<u8>,
    pub amplitude: C,
    pub probability: f64,
    pub address: DimensionalAddress,
}

// ── SparseQuantumRegister ───────────────────────────────────────────

/// A cascade-addressed sparse quantum register.
///
/// Stores only basis states with non-zero amplitude.  Memory scales
/// with actual entanglement complexity, not 2^n.
///
/// No architectural qubit limit — the ceiling is determined by the
/// number of simultaneously populated basis states, not the register width.
pub struct SparseQuantumRegister {
    /// Number of positions (qubits).
    pub n: usize,
    /// Human-readable name.
    pub name: String,
    /// Populated basis states: index → complex amplitude.
    states: HashMap<u64, C>,
    /// Pruning threshold: amplitudes with |a|² below this are removed.
    pub prune_threshold: f64,
}

impl SparseQuantumRegister {
    // ── Constructors ────────────────────────────────────────────────

    /// Create an `n`-position sparse register initialised to |00…0⟩.
    ///
    /// Unlike the dense register, `n` has no hard upper limit.
    /// Memory is O(populated states), not O(2^n).
    pub fn new(n: usize, name: &str) -> Self {
        assert!(n > 0, "register width must be >= 1");
        let mut states = HashMap::new();
        states.insert(0u64, C_ONE);
        Self {
            n,
            name: name.into(),
            states,
            prune_threshold: 1e-15,
        }
    }

    /// Create a sparse register initialised to basis state |value⟩.
    pub fn from_int(n: usize, value: u64, name: &str) -> Self {
        assert!(n > 0);
        assert!(
            n >= 64 || value < (1u64 << n),
            "value exceeds register width"
        );
        let mut states = HashMap::new();
        states.insert(value, C_ONE);
        Self {
            n,
            name: name.into(),
            states,
            prune_threshold: 1e-15,
        }
    }

    /// Create a sparse register from a bit pattern (MSB first).
    pub fn from_bits(bits: &[u8], name: &str) -> Self {
        Self::from_int(bits.len(), bits_to_index(bits), name)
    }

    /// Set the pruning threshold.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.prune_threshold = threshold;
        self
    }

    /// Number of currently populated basis states.
    pub fn population(&self) -> usize {
        self.states.len()
    }

    /// Maximum possible basis states (2^n), returned as u128 to handle large n.
    pub fn hilbert_dim(&self) -> u128 {
        1u128 << self.n
    }

    /// Sparsity ratio: populated / 2^n.
    pub fn sparsity(&self) -> f64 {
        if self.n >= 64 {
            // For very large registers, this is effectively 0
            self.states.len() as f64 / (u64::MAX as f64)
        } else {
            self.states.len() as f64 / (1u64 << self.n) as f64
        }
    }

    // ── Internal helpers ────────────────────────────────────────────

    /// Get amplitude for a basis state (zero if not populated).
    fn get(&self, index: u64) -> C {
        self.states.get(&index).copied().unwrap_or(C_ZERO)
    }

    /// Set amplitude for a basis state.  Inserts if non-zero, removes if zero.
    fn set(&mut self, index: u64, amp: C) {
        if c_norm_sq(amp) > self.prune_threshold {
            self.states.insert(index, amp);
        } else {
            self.states.remove(&index);
        }
    }

    /// Bit mask for position `k` (MSB-first convention).
    fn bit_mask(&self, k: usize) -> u64 {
        1u64 << (self.n - 1 - k)
    }

    // ── Gate machinery ──────────────────────────────────────────────

    /// Apply a 2×2 unitary gate to position `k`.
    ///
    /// For each populated state, we find its partner by flipping bit k.
    /// We process pairs (i, j) where bit k of i is 0.
    fn apply_single(&mut self, k: usize, u: [[C; 2]; 2]) {
        assert!(k < self.n);
        let bit = self.bit_mask(k);

        // Collect all populated indices
        let indices: Vec<u64> = self.states.keys().copied().collect();

        // Find all unique pairs (one with bit=0, partner with bit=1)
        let mut processed: HashMap<u64, C> = HashMap::new();
        let mut seen = std::collections::HashSet::new();

        for &idx in &indices {
            let base = idx & !bit; // clear bit k → the |0⟩ partner
            if seen.contains(&base) {
                continue;
            }
            seen.insert(base);

            let i = base;       // bit k = 0
            let j = base | bit; // bit k = 1

            let a = self.get(i);
            let b = self.get(j);

            let new_i = c_add(c_mul(u[0][0], a), c_mul(u[0][1], b));
            let new_j = c_add(c_mul(u[1][0], a), c_mul(u[1][1], b));

            if c_norm_sq(new_i) > 0.0 {
                processed.insert(i, new_i);
            }
            if c_norm_sq(new_j) > 0.0 {
                processed.insert(j, new_j);
            }
        }

        // Replace all affected entries, remove zeroed ones
        for base in &seen {
            let i = *base;
            let j = i | bit;
            self.states.remove(&i);
            self.states.remove(&j);
        }
        for (idx, amp) in processed {
            if c_norm_sq(amp) > 0.0 {
                self.states.insert(idx, amp);
            }
        }
    }

    /// Apply a controlled 2×2 unitary: when position `c`=|1⟩, apply `u` to position `t`.
    fn apply_controlled(&mut self, c: usize, t: usize, u: [[C; 2]; 2]) {
        assert!(c < self.n && t < self.n && c != t);
        let c_bit = self.bit_mask(c);
        let t_bit = self.bit_mask(t);

        let indices: Vec<u64> = self.states.keys().copied().collect();
        let mut processed: HashMap<u64, C> = HashMap::new();
        let mut seen = std::collections::HashSet::new();

        for &idx in &indices {
            // Only act when control bit is set
            if idx & c_bit == 0 {
                continue;
            }
            let base = idx & !t_bit; // clear target bit
            if seen.contains(&base) {
                continue;
            }
            seen.insert(base);

            let i = base;        // target bit = 0, control = 1
            let j = base | t_bit; // target bit = 1, control = 1

            let a = self.get(i);
            let b = self.get(j);

            let new_i = c_add(c_mul(u[0][0], a), c_mul(u[0][1], b));
            let new_j = c_add(c_mul(u[1][0], a), c_mul(u[1][1], b));

            if c_norm_sq(new_i) > 0.0 {
                processed.insert(i, new_i);
            }
            if c_norm_sq(new_j) > 0.0 {
                processed.insert(j, new_j);
            }
        }

        for base in &seen {
            let i = *base;
            let j = i | t_bit;
            self.states.remove(&i);
            self.states.remove(&j);
        }
        for (idx, amp) in processed {
            if c_norm_sq(amp) > 0.0 {
                self.states.insert(idx, amp);
            }
        }
    }

    /// Apply a doubly-controlled 2×2 unitary.
    fn apply_cc(&mut self, c1: usize, c2: usize, t: usize, u: [[C; 2]; 2]) {
        assert!(c1 != c2 && c1 != t && c2 != t);
        let c1_bit = self.bit_mask(c1);
        let c2_bit = self.bit_mask(c2);
        let t_bit = self.bit_mask(t);

        let indices: Vec<u64> = self.states.keys().copied().collect();
        let mut processed: HashMap<u64, C> = HashMap::new();
        let mut seen = std::collections::HashSet::new();

        for &idx in &indices {
            if idx & c1_bit == 0 || idx & c2_bit == 0 {
                continue;
            }
            let base = idx & !t_bit;
            if seen.contains(&base) {
                continue;
            }
            seen.insert(base);

            let i = base;
            let j = base | t_bit;
            let a = self.get(i);
            let b = self.get(j);

            let new_i = c_add(c_mul(u[0][0], a), c_mul(u[0][1], b));
            let new_j = c_add(c_mul(u[1][0], a), c_mul(u[1][1], b));

            if c_norm_sq(new_i) > 0.0 {
                processed.insert(i, new_i);
            }
            if c_norm_sq(new_j) > 0.0 {
                processed.insert(j, new_j);
            }
        }

        for base in &seen {
            let i = *base;
            let j = i | t_bit;
            self.states.remove(&i);
            self.states.remove(&j);
        }
        for (idx, amp) in processed {
            if c_norm_sq(amp) > 0.0 {
                self.states.insert(idx, amp);
            }
        }
    }

    // ── Single-position gates ───────────────────────────────────────

    /// Hadamard gate.
    pub fn hadamard(&mut self, k: usize) {
        let h = SQRT2_INV;
        self.apply_single(k, [[(h, 0.0), (h, 0.0)], [(h, 0.0), (-h, 0.0)]]);
    }

    /// Pauli-X (NOT).
    pub fn pauli_x(&mut self, k: usize) {
        self.apply_single(k, [[C_ZERO, C_ONE], [C_ONE, C_ZERO]]);
    }

    /// Pauli-Y.
    pub fn pauli_y(&mut self, k: usize) {
        self.apply_single(k, [[C_ZERO, (0.0, -1.0)], [(0.0, 1.0), C_ZERO]]);
    }

    /// Pauli-Z.
    pub fn pauli_z(&mut self, k: usize) {
        self.apply_single(k, [[C_ONE, C_ZERO], [C_ZERO, (-1.0, 0.0)]]);
    }

    /// Phase gate: |1⟩ → e^(iθ)|1⟩.
    pub fn phase_gate(&mut self, k: usize, theta: f64) {
        self.apply_single(k, [[C_ONE, C_ZERO], [C_ZERO, c_phase(theta)]]);
    }

    /// S gate (√Z).
    pub fn s_gate(&mut self, k: usize) {
        self.phase_gate(k, PI / 2.0);
    }

    /// T gate.
    pub fn t_gate(&mut self, k: usize) {
        self.phase_gate(k, PI / 4.0);
    }

    /// Rx rotation.
    pub fn rx(&mut self, k: usize, theta: f64) {
        let c = (theta / 2.0).cos();
        let s = (theta / 2.0).sin();
        self.apply_single(k, [[(c, 0.0), (0.0, -s)], [(0.0, -s), (c, 0.0)]]);
    }

    /// Ry rotation.
    pub fn ry(&mut self, k: usize, theta: f64) {
        let c = (theta / 2.0).cos();
        let s = (theta / 2.0).sin();
        self.apply_single(k, [[(c, 0.0), (-s, 0.0)], [(s, 0.0), (c, 0.0)]]);
    }

    /// Rz rotation.
    pub fn rz(&mut self, k: usize, theta: f64) {
        self.apply_single(
            k,
            [[c_phase(-theta / 2.0), C_ZERO], [C_ZERO, c_phase(theta / 2.0)]],
        );
    }

    // ── Two-position gates ──────────────────────────────────────────

    /// CNOT (Controlled-X).
    pub fn cnot(&mut self, control: usize, target: usize) {
        self.apply_controlled(control, target, [[C_ZERO, C_ONE], [C_ONE, C_ZERO]]);
    }

    /// Controlled-Z.
    pub fn cz(&mut self, a: usize, b: usize) {
        self.apply_controlled(a, b, [[C_ONE, C_ZERO], [C_ZERO, (-1.0, 0.0)]]);
    }

    /// Controlled phase rotation.
    pub fn controlled_phase(&mut self, control: usize, target: usize, theta: f64) {
        self.apply_controlled(
            control,
            target,
            [[C_ONE, C_ZERO], [C_ZERO, c_phase(theta)]],
        );
    }

    /// SWAP two positions.
    pub fn swap(&mut self, a: usize, b: usize) {
        assert!(a < self.n && b < self.n && a != b);
        let a_bit = self.bit_mask(a);
        let b_bit = self.bit_mask(b);

        let indices: Vec<u64> = self.states.keys().copied().collect();
        let mut seen = std::collections::HashSet::new();
        let mut swaps = Vec::new();

        for &idx in &indices {
            let a_set = (idx & a_bit) != 0;
            let b_set = (idx & b_bit) != 0;
            if a_set != b_set {
                let partner = idx ^ a_bit ^ b_bit;
                let key = idx.min(partner);
                if seen.insert(key) {
                    swaps.push((idx, partner));
                }
            }
        }

        for (i, j) in swaps {
            let ai = self.get(i);
            let aj = self.get(j);
            // Use insert/remove directly to handle zero amplitudes correctly
            if c_norm_sq(aj) > 0.0 {
                self.states.insert(i, aj);
            } else {
                self.states.remove(&i);
            }
            if c_norm_sq(ai) > 0.0 {
                self.states.insert(j, ai);
            } else {
                self.states.remove(&j);
            }
        }
    }

    // ── Three-position gates ────────────────────────────────────────

    /// Toffoli (CCX).
    pub fn toffoli(&mut self, c1: usize, c2: usize, target: usize) {
        self.apply_cc(c1, c2, target, [[C_ZERO, C_ONE], [C_ONE, C_ZERO]]);
    }

    /// Fredkin (Controlled-SWAP).
    pub fn fredkin(&mut self, control: usize, t1: usize, t2: usize) {
        assert!(control != t1 && control != t2 && t1 != t2);
        let c_bit = self.bit_mask(control);
        let t1_bit = self.bit_mask(t1);
        let t2_bit = self.bit_mask(t2);

        let indices: Vec<u64> = self.states.keys().copied().collect();
        let mut seen = std::collections::HashSet::new();
        let mut swaps = Vec::new();

        for &idx in &indices {
            if idx & c_bit == 0 {
                continue;
            }
            let t1_set = (idx & t1_bit) != 0;
            let t2_set = (idx & t2_bit) != 0;
            if t1_set != t2_set {
                let partner = idx ^ t1_bit ^ t2_bit;
                let key = idx.min(partner);
                if seen.insert(key) {
                    swaps.push((idx, partner));
                }
            }
        }

        for (i, j) in swaps {
            let ai = self.get(i);
            let aj = self.get(j);
            if c_norm_sq(aj) > 0.0 {
                self.states.insert(i, aj);
            } else {
                self.states.remove(&i);
            }
            if c_norm_sq(ai) > 0.0 {
                self.states.insert(j, ai);
            } else {
                self.states.remove(&j);
            }
        }
    }

    // ── Composite operations ────────────────────────────────────────

    /// Quantum Fourier Transform on the given positions.
    pub fn qft(&mut self, positions: &[usize]) {
        let m = positions.len();
        for j in 0..m {
            self.hadamard(positions[j]);
            for k in (j + 1)..m {
                let angle = 2.0 * PI / (1u64 << (k - j + 1)) as f64;
                self.controlled_phase(positions[k], positions[j], angle);
            }
        }
        for i in 0..m / 2 {
            self.swap(positions[i], positions[m - 1 - i]);
        }
    }

    /// Inverse QFT.
    pub fn inverse_qft(&mut self, positions: &[usize]) {
        let m = positions.len();
        for i in 0..m / 2 {
            self.swap(positions[i], positions[m - 1 - i]);
        }
        for j in (0..m).rev() {
            for k in ((j + 1)..m).rev() {
                let angle = -2.0 * PI / (1u64 << (k - j + 1)) as f64;
                self.controlled_phase(positions[k], positions[j], angle);
            }
            self.hadamard(positions[j]);
        }
    }

    /// Grover diffusion operator on all positions.
    pub fn grover_diffusion(&mut self) {
        for k in 0..self.n {
            self.hadamard(k);
        }
        // Phase-flip everything except |0…0⟩
        let indices: Vec<u64> = self.states.keys().copied().collect();
        for idx in indices {
            if idx != 0 {
                if let Some(amp) = self.states.get_mut(&idx) {
                    *amp = c_scale(-1.0, *amp);
                }
            }
        }
        for k in 0..self.n {
            self.hadamard(k);
        }
    }

    /// Phase-flip all basis states where the predicate returns true.
    pub fn apply_oracle(&mut self, predicate: &dyn Fn(&[u8]) -> bool) {
        let indices: Vec<u64> = self.states.keys().copied().collect();
        for idx in indices {
            let bits = index_to_bits(idx, self.n);
            if predicate(&bits) {
                if let Some(amp) = self.states.get_mut(&idx) {
                    *amp = c_scale(-1.0, *amp);
                }
            }
        }
    }

    /// Reversible function evaluation: |x⟩|y⟩ → |x⟩|y ⊕ f(x)⟩.
    pub fn apply_function(
        &mut self,
        input_range: (usize, usize),
        output_range: (usize, usize),
        f: &dyn Fn(u64) -> u64,
    ) {
        let in_len = input_range.1 - input_range.0;
        let out_len = output_range.1 - output_range.0;

        let entries: Vec<(u64, C)> = self.states.drain().collect();
        let mut new_states: HashMap<u64, C> = HashMap::new();

        for (i, amp) in entries {
            let x = extract_field(i, input_range.0, in_len, self.n);
            let y = extract_field(i, output_range.0, out_len, self.n);
            let fx = f(x) & ((1u64 << out_len) - 1);
            let new_y = y ^ fx;
            let j = replace_field(i, output_range.0, out_len, new_y, self.n);
            let entry = new_states.entry(j).or_insert(C_ZERO);
            *entry = c_add(*entry, amp);
        }

        self.states = new_states;
    }

    /// Controlled permutation on a work sub-register.
    pub fn controlled_permutation(
        &mut self,
        control: usize,
        work_start: usize,
        work_size: usize,
        perm: &dyn Fn(u64) -> u64,
    ) {
        let c_bit = self.bit_mask(control);

        let entries: Vec<(u64, C)> = self.states.drain().collect();
        let mut new_states: HashMap<u64, C> = HashMap::new();

        for (i, amp) in entries {
            if i & c_bit == 0 {
                let entry = new_states.entry(i).or_insert(C_ZERO);
                *entry = c_add(*entry, amp);
            } else {
                let work_val = extract_field(i, work_start, work_size, self.n);
                let new_work = perm(work_val);
                let j = replace_field(i, work_start, work_size, new_work, self.n);
                let entry = new_states.entry(j).or_insert(C_ZERO);
                *entry = c_add(*entry, amp);
            }
        }

        self.states = new_states;
    }

    // ── Pruning & Normalization ─────────────────────────────────────

    /// Remove all basis states with |amplitude|² below `threshold`,
    /// then renormalise.  Returns the number of states pruned.
    pub fn prune(&mut self, threshold: f64) -> usize {
        let before = self.states.len();
        self.states.retain(|_, amp| c_norm_sq(*amp) >= threshold);
        let pruned = before - self.states.len();
        if pruned > 0 {
            self.normalize();
        }
        pruned
    }

    /// Auto-prune using the register's configured threshold.
    pub fn auto_prune(&mut self) -> usize {
        let t = self.prune_threshold;
        self.prune(t)
    }

    /// Renormalise amplitudes so total probability = 1.
    pub fn normalize(&mut self) {
        let total: f64 = self.states.values().map(|a| c_norm_sq(*a)).sum();
        if total > 1e-30 && (total - 1.0).abs() > 1e-12 {
            let factor = 1.0 / total.sqrt();
            for amp in self.states.values_mut() {
                *amp = c_scale(factor, *amp);
            }
        }
    }

    /// Total probability (should be 1.0 if normalised).
    pub fn total_probability(&self) -> f64 {
        self.states.values().map(|a| c_norm_sq(*a)).sum()
    }

    // ── Measurement ─────────────────────────────────────────────────

    /// Probability distribution — only for populated states.
    pub fn probabilities(&self) -> Vec<(u64, f64)> {
        self.states
            .iter()
            .map(|(&idx, amp)| (idx, c_norm_sq(*amp)))
            .collect()
    }

    /// Measure a single position (destructive).
    pub fn measure_position(&mut self, k: usize, seed: u64) -> PositionMeasurement {
        assert!(k < self.n);
        let bit = self.bit_mask(k);

        let mut p0 = 0.0;
        for (&idx, amp) in &self.states {
            if idx & bit == 0 {
                p0 += c_norm_sq(*amp);
            }
        }

        let mut rng = seed ^ (k as u64).wrapping_mul(0x517cc1b727220a95);
        let r = splitmix64(&mut rng);
        let value = if r < p0 { 0u8 } else { 1u8 };
        let prob = if value == 0 { p0 } else { 1.0 - p0 };

        // Collapse: remove states inconsistent with measurement
        let keep_set = if value == 0 { 0u64 } else { bit };
        self.states.retain(|&idx, _| (idx & bit) == keep_set);
        self.normalize();

        PositionMeasurement {
            position: k,
            value,
            probability: prob,
        }
    }

    /// Measure all positions (destructive).
    pub fn measure_all(&mut self, seed: u64) -> FullMeasurement {
        let probs = self.probabilities();
        let mut rng = seed;
        let r = splitmix64(&mut rng);

        let mut cumulative = 0.0;
        let mut chosen = probs.last().map(|(idx, _)| *idx).unwrap_or(0);
        let mut chosen_prob = 0.0;

        // Sort by index for deterministic behaviour
        let mut sorted_probs = probs;
        sorted_probs.sort_by_key(|(idx, _)| *idx);

        for &(idx, prob) in &sorted_probs {
            cumulative += prob;
            if r < cumulative {
                chosen = idx;
                chosen_prob = prob;
                break;
            }
        }

        let bits = index_to_bits(chosen, self.n);

        // Collapse to the chosen state
        self.states.clear();
        self.states.insert(chosen, C_ONE);

        FullMeasurement {
            bits,
            index: chosen,
            probability: chosen_prob,
        }
    }

    /// Non-destructive sample (MDB exclusive).
    pub fn sample_all(&self, seed: u64) -> FullMeasurement {
        let mut probs: Vec<(u64, f64)> = self.probabilities();
        probs.sort_by_key(|(idx, _)| *idx);

        let mut rng = seed;
        let r = splitmix64(&mut rng);

        let mut cumulative = 0.0;
        let mut chosen = probs.last().map(|(idx, _)| *idx).unwrap_or(0);
        let mut chosen_prob = 0.0;

        for &(idx, prob) in &probs {
            cumulative += prob;
            if r < cumulative {
                chosen = idx;
                chosen_prob = prob;
                break;
            }
        }

        FullMeasurement {
            bits: index_to_bits(chosen, self.n),
            index: chosen,
            probability: chosen_prob,
        }
    }

    // ── Non-destructive operations (MDB exclusive) ──────────────────

    /// Peek at the full register state without modification.
    pub fn peek(&self) -> RegisterView {
        let mut states = Vec::new();

        let mut sorted: Vec<(u64, C)> = self.states.iter().map(|(&k, &v)| (k, v)).collect();
        sorted.sort_by_key(|(idx, _)| *idx);

        for &(idx, amp) in &sorted {
            let prob = c_norm_sq(amp);
            if prob > 1e-15 {
                let phase = amp.1.atan2(amp.0);
                states.push(RegisterStateView {
                    index: idx,
                    bits: index_to_bits(idx, self.n),
                    probability: prob,
                    phase,
                    label: format!("|{:0width$b}⟩", idx, width = self.n),
                });
            }
        }

        RegisterView {
            n: self.n,
            total_states: if self.n < 64 { 1usize << self.n } else { usize::MAX },
            nonzero_states: states.len(),
            states,
        }
    }

    /// Fork: create an independent copy.
    pub fn fork(&self) -> SparseQuantumRegister {
        SparseQuantumRegister {
            n: self.n,
            name: format!("{}_fork", self.name),
            states: self.states.clone(),
            prune_threshold: self.prune_threshold,
        }
    }

    // ── Cascade integration ─────────────────────────────────────────

    /// Compute the dimensional cascade address for every populated basis state.
    ///
    /// Returns entries sorted by basis index.
    pub fn cascade_snapshot(&self) -> Vec<CascadeEntry> {
        let mut entries: Vec<CascadeEntry> = self
            .states
            .iter()
            .map(|(&idx, &amp)| {
                let bits = index_to_bits(idx, self.n);
                let address = DimensionalAddress::from_bits(&bits);
                CascadeEntry {
                    index: idx,
                    bits,
                    amplitude: amp,
                    probability: c_norm_sq(amp),
                    address,
                }
            })
            .collect();
        entries.sort_by_key(|e| e.index);
        entries
    }

    // ── Utility ─────────────────────────────────────────────────────

    /// Count of populated basis states.
    pub fn nonzero_count(&self) -> usize {
        self.states.len()
    }

    /// Reset to |0…0⟩.
    pub fn reset(&mut self) {
        self.states.clear();
        self.states.insert(0, C_ONE);
    }

    /// Raw amplitude at basis state `index`.
    pub fn amplitude(&self, index: u64) -> (f64, f64) {
        self.get(index)
    }

    /// Set amplitude directly (caller must re-normalise).
    pub fn set_amplitude(&mut self, index: u64, amp: (f64, f64)) {
        self.set(index, amp);
    }

    /// State fidelity with another sparse register: |⟨self|other⟩|².
    pub fn fidelity(&self, other: &SparseQuantumRegister) -> f64 {
        assert_eq!(self.n, other.n);
        let mut inner = C_ZERO;

        // Iterate over the smaller map
        if self.states.len() <= other.states.len() {
            for (&idx, &amp_self) in &self.states {
                let amp_other = other.get(idx);
                let conj = (amp_self.0, -amp_self.1);
                inner = c_add(inner, c_mul(conj, amp_other));
            }
        } else {
            for (&idx, &amp_other) in &other.states {
                let amp_self = self.get(idx);
                let conj = (amp_self.0, -amp_self.1);
                inner = c_add(inner, c_mul(conj, amp_other));
            }
        }

        c_norm_sq(inner)
    }

    /// Memory usage estimate in bytes.
    pub fn memory_bytes(&self) -> usize {
        // Each entry: u64 key (8) + (f64, f64) value (16) + HashMap overhead (~32)
        self.states.len() * 56 + std::mem::size_of::<Self>()
    }
}

// ── Display ─────────────────────────────────────────────────────────

impl fmt::Display for SparseQuantumRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "SparseRegister '{}' ({} positions, {} populated, {:.2e} sparsity):",
            self.name,
            self.n,
            self.states.len(),
            self.sparsity()
        )?;

        let mut sorted: Vec<(u64, C)> = self.states.iter().map(|(&k, &v)| (k, v)).collect();
        sorted.sort_by_key(|(idx, _)| *idx);

        let show = sorted.len().min(32);
        for &(idx, amp) in &sorted[..show] {
            let prob = c_norm_sq(amp);
            if prob > 1e-10 {
                let phase = amp.1.atan2(amp.0);
                writeln!(
                    f,
                    "  |{:0width$b}⟩  p={:.6}  φ={:.4}",
                    idx,
                    prob,
                    phase,
                    width = self.n
                )?;
            }
        }
        if sorted.len() > show {
            writeln!(f, "  ... and {} more states", sorted.len() - show)?;
        }
        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-10
    }

    // ── Basic construction ──────────────────────────────────────────

    #[test]
    fn test_new_sparse() {
        let r = SparseQuantumRegister::new(3, "t");
        assert_eq!(r.n, 3);
        assert_eq!(r.population(), 1);
        assert!(approx(c_norm_sq(r.amplitude(0)), 1.0));
    }

    #[test]
    fn test_from_int() {
        let r = SparseQuantumRegister::from_int(3, 5, "t");
        assert!(approx(c_norm_sq(r.amplitude(5)), 1.0));
        assert!(approx(c_norm_sq(r.amplitude(0)), 0.0));
        assert_eq!(r.population(), 1);
    }

    #[test]
    fn test_from_bits() {
        let r = SparseQuantumRegister::from_bits(&[1, 0, 1], "t");
        assert!(approx(c_norm_sq(r.amplitude(5)), 1.0));
    }

    // ── Single-qubit gates ──────────────────────────────────────────

    #[test]
    fn test_hadamard_superposition() {
        let mut r = SparseQuantumRegister::new(1, "t");
        r.hadamard(0);
        assert!(approx(c_norm_sq(r.amplitude(0)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(1)), 0.5));
        assert_eq!(r.population(), 2);
    }

    #[test]
    fn test_hadamard_involution() {
        let mut r = SparseQuantumRegister::new(2, "t");
        r.hadamard(0);
        r.hadamard(0);
        assert!(approx(c_norm_sq(r.amplitude(0)), 1.0));
        assert_eq!(r.population(), 1);
    }

    #[test]
    fn test_pauli_x() {
        let mut r = SparseQuantumRegister::new(2, "t");
        r.pauli_x(1);
        assert!(approx(c_norm_sq(r.amplitude(0b01)), 1.0));
        assert_eq!(r.population(), 1);
    }

    #[test]
    fn test_pauli_z_phase() {
        let mut r = SparseQuantumRegister::new(1, "t");
        r.hadamard(0);
        r.pauli_z(0);
        r.hadamard(0);
        assert!(approx(c_norm_sq(r.amplitude(1)), 1.0));
    }

    // ── Two-qubit gates ─────────────────────────────────────────────

    #[test]
    fn test_bell_state() {
        let mut r = SparseQuantumRegister::new(2, "bell");
        r.hadamard(0);
        r.cnot(0, 1);
        assert!(approx(c_norm_sq(r.amplitude(0b00)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(0b11)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(0b01)), 0.0));
        assert!(approx(c_norm_sq(r.amplitude(0b10)), 0.0));
        assert_eq!(r.population(), 2); // Only 2 states populated!
    }

    #[test]
    fn test_swap() {
        let mut r = SparseQuantumRegister::from_bits(&[1, 0], "t");
        r.swap(0, 1);
        assert!(approx(c_norm_sq(r.amplitude(0b01)), 1.0));
    }

    // ── Three-qubit gates ───────────────────────────────────────────

    #[test]
    fn test_toffoli() {
        let mut r = SparseQuantumRegister::from_bits(&[1, 1, 0], "t");
        r.toffoli(0, 1, 2);
        assert!(approx(c_norm_sq(r.amplitude(0b111)), 1.0));

        let mut r2 = SparseQuantumRegister::from_bits(&[1, 0, 0], "t");
        r2.toffoli(0, 1, 2);
        assert!(approx(c_norm_sq(r2.amplitude(0b100)), 1.0));
    }

    #[test]
    fn test_fredkin() {
        let mut r = SparseQuantumRegister::from_bits(&[1, 1, 0], "t");
        r.fredkin(0, 1, 2);
        assert!(approx(c_norm_sq(r.amplitude(0b101)), 1.0));
    }

    // ── Composite operations ────────────────────────────────────────

    #[test]
    fn test_ghz_state() {
        let mut r = SparseQuantumRegister::new(3, "ghz");
        r.hadamard(0);
        r.cnot(0, 1);
        r.cnot(1, 2);
        assert!(approx(c_norm_sq(r.amplitude(0b000)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(0b111)), 0.5));
        assert_eq!(r.population(), 2); // Sparse advantage!
    }

    #[test]
    fn test_qft_uniform() {
        let mut r = SparseQuantumRegister::new(2, "qft");
        r.qft(&[0, 1]);
        for i in 0..4u64 {
            assert!(approx(c_norm_sq(r.amplitude(i)), 0.25));
        }
    }

    #[test]
    fn test_qft_inverse_identity() {
        let mut r = SparseQuantumRegister::from_int(3, 5, "t");
        let original = r.fork();
        r.qft(&[0, 1, 2]);
        r.inverse_qft(&[0, 1, 2]);
        assert!(r.fidelity(&original) > 1.0 - 1e-10);
    }

    // ── Oracle and Grover ───────────────────────────────────────────

    #[test]
    fn test_oracle_phase_flip() {
        let mut r = SparseQuantumRegister::new(2, "t");
        for k in 0..2 {
            r.hadamard(k);
        }
        r.apply_oracle(&|bits: &[u8]| bits[0] == 1 && bits[1] == 0);
        let a_marked = r.amplitude(0b10);
        let a_other = r.amplitude(0b00);
        let phase_diff = (a_marked.1.atan2(a_marked.0) - a_other.1.atan2(a_other.0)).abs();
        assert!(phase_diff > 3.0);
    }

    #[test]
    fn test_grover_2qubit() {
        let mut r = SparseQuantumRegister::new(2, "t");
        for k in 0..2 {
            r.hadamard(k);
        }
        r.apply_oracle(&|bits: &[u8]| bits[0] == 1 && bits[1] == 1);
        r.grover_diffusion();
        assert!(c_norm_sq(r.amplitude(0b11)) > 0.9);
    }

    // ── Non-destructive ops ─────────────────────────────────────────

    #[test]
    fn test_peek_nondestructive() {
        let mut r = SparseQuantumRegister::new(2, "t");
        r.hadamard(0);
        r.cnot(0, 1);
        let v1 = r.peek();
        let v2 = r.peek();
        assert_eq!(v1.nonzero_states, v2.nonzero_states);
        assert!(approx(r.total_probability(), 1.0));
    }

    #[test]
    fn test_fork_independence() {
        let mut r = SparseQuantumRegister::new(2, "orig");
        r.hadamard(0);
        let mut f = r.fork();
        f.pauli_x(1);
        assert!(approx(c_norm_sq(r.amplitude(0b00)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(0b10)), 0.5));
    }

    // ── Normalization ───────────────────────────────────────────────

    #[test]
    fn test_normalization_preserved() {
        let mut r = SparseQuantumRegister::new(3, "t");
        r.hadamard(0);
        assert!(approx(r.total_probability(), 1.0));
        r.cnot(0, 1);
        assert!(approx(r.total_probability(), 1.0));
        r.toffoli(0, 1, 2);
        assert!(approx(r.total_probability(), 1.0));
        r.qft(&[0, 1, 2]);
        assert!(approx(r.total_probability(), 1.0));
    }

    // ── Pruning ─────────────────────────────────────────────────────

    #[test]
    fn test_prune_removes_negligible() {
        let mut r = SparseQuantumRegister::new(2, "t");
        r.hadamard(0);
        r.hadamard(1);
        // 4 states, each p=0.25. Prune above 0.25 should remove nothing.
        let pruned = r.prune(0.24);
        assert_eq!(pruned, 0);
        assert_eq!(r.population(), 4);

        // Prune above 0.26 should remove all → but then renorm would fail.
        // Instead: set a tiny amplitude manually and prune it
        r.states.insert(99, (1e-8, 0.0));
        assert_eq!(r.population(), 5);
        let pruned = r.prune(1e-14);
        assert_eq!(pruned, 1); // removed the 1e-8 state
        assert_eq!(r.population(), 4);
    }

    #[test]
    fn test_auto_prune() {
        let mut r = SparseQuantumRegister::new(1, "t").with_threshold(0.1);
        r.hadamard(0);
        // |0⟩ and |1⟩ each have p=0.5, both above 0.1
        let pruned = r.auto_prune();
        assert_eq!(pruned, 0);
        assert_eq!(r.population(), 2);
    }

    // ── Measurement ─────────────────────────────────────────────────

    #[test]
    fn test_measure_deterministic() {
        let mut r1 = SparseQuantumRegister::new(2, "a");
        r1.hadamard(0);
        r1.cnot(0, 1);
        let mut r2 = r1.fork();
        let m1 = r1.measure_all(42);
        let m2 = r2.measure_all(42);
        assert_eq!(m1.bits, m2.bits);
    }

    #[test]
    fn test_sample_nondestructive() {
        let mut r = SparseQuantumRegister::new(2, "t");
        r.hadamard(0);
        r.cnot(0, 1);
        let s = r.sample_all(42);
        assert_eq!(r.nonzero_count(), 2);
        assert!(s.bits == vec![0, 0] || s.bits == vec![1, 1]);
    }

    // ── Function application ────────────────────────────────────────

    #[test]
    fn test_apply_function_identity() {
        let mut r = SparseQuantumRegister::new(4, "t");
        r.hadamard(0);
        r.hadamard(1);
        r.apply_function((0, 2), (2, 4), &|x| x);
        let view = r.peek();
        assert_eq!(view.nonzero_states, 4);
        for sv in &view.states {
            assert_eq!(sv.bits[0], sv.bits[2]);
            assert_eq!(sv.bits[1], sv.bits[3]);
        }
    }

    #[test]
    fn test_controlled_phase() {
        let mut r = SparseQuantumRegister::from_bits(&[1, 1], "t");
        r.controlled_phase(0, 1, PI);
        let amp = r.amplitude(0b11);
        assert!(approx(amp.0, -1.0));
        assert!(amp.1.abs() < 1e-10);
    }

    // ── Cascade integration ─────────────────────────────────────────

    #[test]
    fn test_cascade_snapshot() {
        let mut r = SparseQuantumRegister::new(3, "t");
        r.hadamard(0);
        r.cnot(0, 1);
        let snap = r.cascade_snapshot();
        assert_eq!(snap.len(), 2);
        // Each entry has a valid address
        for entry in &snap {
            assert_eq!(entry.bits.len(), 3);
            assert!(entry.probability > 0.0);
            assert_eq!(entry.address.n, 3);
        }
    }

    // ── Sparsity metrics ────────────────────────────────────────────

    #[test]
    fn test_sparsity_metrics() {
        let mut r = SparseQuantumRegister::new(10, "t");
        r.hadamard(0);
        r.cnot(0, 1);
        // GHZ-like: only 2 states out of 1024
        assert_eq!(r.population(), 2);
        assert!(r.sparsity() < 0.003); // 2/1024
        assert!(r.memory_bytes() < 1024); // ~112 bytes vs 16KB dense
    }

    // ── Fidelity ────────────────────────────────────────────────────

    #[test]
    fn test_fidelity_identical() {
        let mut r = SparseQuantumRegister::new(2, "t");
        r.hadamard(0);
        r.cnot(0, 1);
        let f = r.fork();
        assert!(approx(r.fidelity(&f), 1.0));
    }

    #[test]
    fn test_fidelity_orthogonal() {
        let r0 = SparseQuantumRegister::from_int(2, 0, "a");
        let r1 = SparseQuantumRegister::from_int(2, 1, "b");
        assert!(approx(r0.fidelity(&r1), 0.0));
    }

    // ── Display ─────────────────────────────────────────────────────

    #[test]
    fn test_display() {
        let mut r = SparseQuantumRegister::new(2, "disp");
        r.hadamard(0);
        let s = format!("{}", r);
        assert!(s.contains("disp"));
        assert!(s.contains("Sparse"));
    }

    // ── Large register (beyond 24 qubits) ───────────────────────────

    #[test]
    fn test_large_register_30_qubits() {
        // This would require 16GB dense. Sparse: just a few states.
        let mut r = SparseQuantumRegister::new(30, "big");
        r.hadamard(0);
        r.cnot(0, 1);
        r.cnot(1, 2);
        // GHZ state on 30 qubits — only 2 populated states
        assert_eq!(r.population(), 2);
        assert!(approx(c_norm_sq(r.amplitude(0)), 0.5));
        assert!(approx(
            c_norm_sq(r.amplitude(0b111 << 27)),
            0.5
        ));
        assert!(r.memory_bytes() < 256);
    }

    #[test]
    fn test_large_register_50_qubits_ghz() {
        // 50-qubit GHZ: would require ~18 PB dense. Sparse: 2 states.
        let mut r = SparseQuantumRegister::new(50, "massive");
        r.hadamard(0);
        for i in 0..49 {
            r.cnot(i, i + 1);
        }
        assert_eq!(r.population(), 2);
        assert!(approx(r.total_probability(), 1.0));

        let snap = r.cascade_snapshot();
        assert_eq!(snap.len(), 2);
    }

    #[test]
    fn test_sparse_circuits_beyond_24() {
        // Circuits that STAY sparse scale well past 24 qubits.
        // GHZ on 28 qubits: only 2 populated states.
        let n = 28;
        let mut r = SparseQuantumRegister::new(n, "sparse28");
        r.hadamard(0);
        for i in 1..n {
            r.cnot(i - 1, i);
        }
        assert_eq!(r.population(), 2);
        assert!(approx(r.total_probability(), 1.0));

        // Partial Hadamard: H on 3 qubits → 8 states, then controlled ops
        let mut r2 = SparseQuantumRegister::new(n, "partial28");
        for k in 0..3 {
            r2.hadamard(k);
        }
        assert_eq!(r2.population(), 8);
        r2.cnot(0, 3);
        assert!(r2.population() <= 16);
    }
}
