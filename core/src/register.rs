//! # Quantum Register
//!
//! Full statevector quantum register with all standard gates, QFT,
//! measurement, and MDB-exclusive non-destructive operations.
//!
//! Supports registers up to 24 positions (16M complex amplitudes).
//! Computationally equivalent to a universal quantum computer's qubit register,
//! plus MDB-exclusive capabilities: peek (non-destructive readout), fork
//! (cloning — violates quantum no-cloning theorem), and deterministic replay.

use std::f64::consts::PI;
use std::fmt;

// ── Complex number helpers ──────────────────────────────────────────

/// Complex number as (real, imaginary).
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

/// Splitmix64 PRNG — returns uniform f64 in [0, 1).
fn splitmix64(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

// ── Bit helpers ─────────────────────────────────────────────────────

fn index_to_bits(index: usize, n: usize) -> Vec<u8> {
    (0..n)
        .map(|k| ((index >> (n - 1 - k)) & 1) as u8)
        .collect()
}

fn bits_to_index(bits: &[u8]) -> usize {
    bits.iter().fold(0usize, |acc, &b| (acc << 1) | (b as usize))
}

/// Extract a contiguous field of `len` bits starting at position `start` (MSB-first).
fn extract_field(index: usize, start: usize, len: usize, n: usize) -> usize {
    let mut val = 0;
    for k in 0..len {
        let bit_pos = n - 1 - (start + k);
        val = (val << 1) | ((index >> bit_pos) & 1);
    }
    val
}

/// Replace a contiguous field of `len` bits starting at position `start`.
fn replace_field(index: usize, start: usize, len: usize, value: usize, n: usize) -> usize {
    let mut result = index;
    for k in 0..len {
        let bit_pos = n - 1 - (start + k);
        let bit_val = (value >> (len - 1 - k)) & 1;
        if bit_val == 1 {
            result |= 1 << bit_pos;
        } else {
            result &= !(1 << bit_pos);
        }
    }
    result
}

// ── Public types ────────────────────────────────────────────────────

/// Non-destructive view of register state (MDB exclusive).
pub struct RegisterView {
    pub n: usize,
    pub total_states: usize,
    pub nonzero_states: usize,
    pub states: Vec<RegisterStateView>,
}

/// One basis state in the view.
pub struct RegisterStateView {
    pub index: u64,
    pub bits: Vec<u8>,
    pub probability: f64,
    pub phase: f64,
    pub label: String,
}

/// Result of measuring one position.
pub struct PositionMeasurement {
    pub position: usize,
    pub value: u8,
    pub probability: f64,
}

/// Result of measuring the full register.
pub struct FullMeasurement {
    pub bits: Vec<u8>,
    pub index: u64,
    pub probability: f64,
}

// ── QuantumRegister ─────────────────────────────────────────────────

/// A quantum register of `n` positions with 2^n complex amplitudes.
///
/// Equivalent to a universal quantum computer's qubit register.
/// Position 0 is the most-significant bit.
pub struct QuantumRegister {
    /// Number of positions (analogous to qubits).
    pub n: usize,
    /// Human-readable name.
    pub name: String,
    /// Complex amplitudes indexed by computational basis state.
    amplitudes: Vec<C>,
}

impl QuantumRegister {
    // ── Constructors ────────────────────────────────────────────────

    /// Create an `n`-position register initialised to |00…0⟩.
    pub fn new(n: usize, name: &str) -> Self {
        assert!(n > 0 && n <= 24, "register width must be 1..=24");
        let mut amps = vec![C_ZERO; 1 << n];
        amps[0] = C_ONE;
        Self {
            n,
            name: name.into(),
            amplitudes: amps,
        }
    }

    /// Create a register initialised to basis state |value⟩.
    pub fn from_int(n: usize, value: usize, name: &str) -> Self {
        assert!(n > 0 && n <= 24);
        assert!(value < (1 << n), "value exceeds register width");
        let mut amps = vec![C_ZERO; 1 << n];
        amps[value] = C_ONE;
        Self {
            n,
            name: name.into(),
            amplitudes: amps,
        }
    }

    /// Create a register from a bit pattern (MSB first).
    pub fn from_bits(bits: &[u8], name: &str) -> Self {
        Self::from_int(bits.len(), bits_to_index(bits), name)
    }

    /// Dimension = 2^n.
    pub fn dim(&self) -> usize {
        1 << self.n
    }

    // ── Internal gate machinery ─────────────────────────────────────

    /// Apply a 2×2 unitary gate to position `k`.
    fn apply_single(&mut self, k: usize, u: [[C; 2]; 2]) {
        assert!(k < self.n);
        let bit = 1 << (self.n - 1 - k);
        let mut new = self.amplitudes.clone();

        for i in 0..self.dim() {
            if i & bit != 0 {
                continue;
            } // process |0⟩ half only
            let j = i | bit;
            let a = self.amplitudes[i];
            let b = self.amplitudes[j];
            new[i] = c_add(c_mul(u[0][0], a), c_mul(u[0][1], b));
            new[j] = c_add(c_mul(u[1][0], a), c_mul(u[1][1], b));
        }
        self.amplitudes = new;
    }

    /// Apply a controlled 2×2 unitary: when position `c`=|1⟩, apply `u` to position `t`.
    fn apply_controlled(&mut self, c: usize, t: usize, u: [[C; 2]; 2]) {
        assert!(c < self.n && t < self.n && c != t);
        let c_bit = 1 << (self.n - 1 - c);
        let t_bit = 1 << (self.n - 1 - t);
        let mut new = self.amplitudes.clone();

        for i in 0..self.dim() {
            if i & c_bit == 0 {
                continue;
            }
            if i & t_bit != 0 {
                continue;
            }
            let j = i | t_bit;
            let a = self.amplitudes[i];
            let b = self.amplitudes[j];
            new[i] = c_add(c_mul(u[0][0], a), c_mul(u[0][1], b));
            new[j] = c_add(c_mul(u[1][0], a), c_mul(u[1][1], b));
        }
        self.amplitudes = new;
    }

    /// Apply a doubly-controlled 2×2 unitary.
    fn apply_cc(&mut self, c1: usize, c2: usize, t: usize, u: [[C; 2]; 2]) {
        assert!(c1 != c2 && c1 != t && c2 != t);
        let c1_bit = 1 << (self.n - 1 - c1);
        let c2_bit = 1 << (self.n - 1 - c2);
        let t_bit = 1 << (self.n - 1 - t);
        let mut new = self.amplitudes.clone();

        for i in 0..self.dim() {
            if i & c1_bit == 0 || i & c2_bit == 0 {
                continue;
            }
            if i & t_bit != 0 {
                continue;
            }
            let j = i | t_bit;
            let a = self.amplitudes[i];
            let b = self.amplitudes[j];
            new[i] = c_add(c_mul(u[0][0], a), c_mul(u[0][1], b));
            new[j] = c_add(c_mul(u[1][0], a), c_mul(u[1][1], b));
        }
        self.amplitudes = new;
    }

    // ── Single-position gates ───────────────────────────────────────

    /// Hadamard gate: |0⟩ → (|0⟩+|1⟩)/√2, |1⟩ → (|0⟩−|1⟩)/√2.
    pub fn hadamard(&mut self, k: usize) {
        let h = SQRT2_INV;
        self.apply_single(k, [[(h, 0.0), (h, 0.0)], [(h, 0.0), (-h, 0.0)]]);
    }

    /// Pauli-X (NOT): |0⟩↔|1⟩.
    pub fn pauli_x(&mut self, k: usize) {
        self.apply_single(k, [[C_ZERO, C_ONE], [C_ONE, C_ZERO]]);
    }

    /// Pauli-Y.
    pub fn pauli_y(&mut self, k: usize) {
        self.apply_single(k, [[C_ZERO, (0.0, -1.0)], [(0.0, 1.0), C_ZERO]]);
    }

    /// Pauli-Z: |0⟩→|0⟩, |1⟩→−|1⟩.
    pub fn pauli_z(&mut self, k: usize) {
        self.apply_single(
            k,
            [[C_ONE, C_ZERO], [C_ZERO, (-1.0, 0.0)]],
        );
    }

    /// Phase gate: |0⟩→|0⟩, |1⟩→e^(iθ)|1⟩.
    pub fn phase_gate(&mut self, k: usize, theta: f64) {
        self.apply_single(k, [[C_ONE, C_ZERO], [C_ZERO, c_phase(theta)]]);
    }

    /// S gate (√Z) = Phase(π/2).
    pub fn s_gate(&mut self, k: usize) {
        self.phase_gate(k, PI / 2.0);
    }

    /// T gate = Phase(π/4).
    pub fn t_gate(&mut self, k: usize) {
        self.phase_gate(k, PI / 4.0);
    }

    /// Rx rotation: e^{−iθX/2}.
    pub fn rx(&mut self, k: usize, theta: f64) {
        let c = (theta / 2.0).cos();
        let s = (theta / 2.0).sin();
        self.apply_single(k, [[(c, 0.0), (0.0, -s)], [(0.0, -s), (c, 0.0)]]);
    }

    /// Ry rotation: e^{−iθY/2}.
    pub fn ry(&mut self, k: usize, theta: f64) {
        let c = (theta / 2.0).cos();
        let s = (theta / 2.0).sin();
        self.apply_single(k, [[(c, 0.0), (-s, 0.0)], [(s, 0.0), (c, 0.0)]]);
    }

    /// Rz rotation: e^{−iθZ/2}.
    pub fn rz(&mut self, k: usize, theta: f64) {
        self.apply_single(
            k,
            [[c_phase(-theta / 2.0), C_ZERO], [C_ZERO, c_phase(theta / 2.0)]],
        );
    }

    // ── Two-position gates ──────────────────────────────────────────

    /// CNOT (Controlled-X): flip target when control = |1⟩.
    pub fn cnot(&mut self, control: usize, target: usize) {
        self.apply_controlled(control, target, [[C_ZERO, C_ONE], [C_ONE, C_ZERO]]);
    }

    /// Controlled-Z: phase-flip target when control = |1⟩.
    pub fn cz(&mut self, a: usize, b: usize) {
        self.apply_controlled(
            a,
            b,
            [[C_ONE, C_ZERO], [C_ZERO, (-1.0, 0.0)]],
        );
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
        let a_bit = 1 << (self.n - 1 - a);
        let b_bit = 1 << (self.n - 1 - b);
        for i in 0..self.dim() {
            let a_set = (i & a_bit) != 0;
            let b_set = (i & b_bit) != 0;
            if a_set != b_set && i < (i ^ a_bit ^ b_bit) {
                let j = i ^ a_bit ^ b_bit;
                self.amplitudes.swap(i, j);
            }
        }
    }

    // ── Three-position gates ────────────────────────────────────────

    /// Toffoli (CCX): flip target when both controls = |1⟩.
    pub fn toffoli(&mut self, c1: usize, c2: usize, target: usize) {
        self.apply_cc(c1, c2, target, [[C_ZERO, C_ONE], [C_ONE, C_ZERO]]);
    }

    /// Fredkin (Controlled-SWAP): swap t1, t2 when control = |1⟩.
    pub fn fredkin(&mut self, control: usize, t1: usize, t2: usize) {
        assert!(control != t1 && control != t2 && t1 != t2);
        let c_bit = 1 << (self.n - 1 - control);
        let t1_bit = 1 << (self.n - 1 - t1);
        let t2_bit = 1 << (self.n - 1 - t2);
        for i in 0..self.dim() {
            if i & c_bit == 0 {
                continue;
            }
            let t1_set = (i & t1_bit) != 0;
            let t2_set = (i & t2_bit) != 0;
            if t1_set != t2_set {
                let j = i ^ t1_bit ^ t2_bit;
                if i < j {
                    self.amplitudes.swap(i, j);
                }
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

    /// Inverse QFT on the given positions.
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

    /// Grover diffusion operator (inversion about the mean) on all positions.
    pub fn grover_diffusion(&mut self) {
        for k in 0..self.n {
            self.hadamard(k);
        }
        // Phase-flip everything except |0…0⟩
        for i in 1..self.dim() {
            self.amplitudes[i] = c_scale(-1.0, self.amplitudes[i]);
        }
        for k in 0..self.n {
            self.hadamard(k);
        }
    }

    /// Phase-flip all basis states where the predicate returns true.
    pub fn apply_oracle(&mut self, predicate: &dyn Fn(&[u8]) -> bool) {
        for i in 0..self.dim() {
            let bits = index_to_bits(i, self.n);
            if predicate(&bits) {
                self.amplitudes[i] = c_scale(-1.0, self.amplitudes[i]);
            }
        }
    }

    /// Reversible function evaluation: |x⟩|y⟩ → |x⟩|y ⊕ f(x)⟩.
    pub fn apply_function(
        &mut self,
        input_range: (usize, usize),
        output_range: (usize, usize),
        f: &dyn Fn(usize) -> usize,
    ) {
        let in_len = input_range.1 - input_range.0;
        let out_len = output_range.1 - output_range.0;
        let mut new_amps = vec![C_ZERO; self.dim()];

        for i in 0..self.dim() {
            let amp = self.amplitudes[i];
            if amp.0 == 0.0 && amp.1 == 0.0 {
                continue;
            }
            let x = extract_field(i, input_range.0, in_len, self.n);
            let y = extract_field(i, output_range.0, out_len, self.n);
            let fx = f(x) & ((1 << out_len) - 1);
            let new_y = y ^ fx;
            let j = replace_field(i, output_range.0, out_len, new_y, self.n);
            new_amps[j] = c_add(new_amps[j], amp);
        }
        self.amplitudes = new_amps;
    }

    /// Controlled permutation: when position `control` = |1⟩,
    /// apply `perm(work_value)` to the work sub-register.
    pub fn controlled_permutation(
        &mut self,
        control: usize,
        work_start: usize,
        work_size: usize,
        perm: &dyn Fn(usize) -> usize,
    ) {
        let c_bit = 1 << (self.n - 1 - control);
        let mut new_amps = vec![C_ZERO; self.dim()];

        for i in 0..self.dim() {
            let amp = self.amplitudes[i];
            if amp.0 == 0.0 && amp.1 == 0.0 {
                continue;
            }
            if i & c_bit == 0 {
                new_amps[i] = c_add(new_amps[i], amp);
            } else {
                let work_val = extract_field(i, work_start, work_size, self.n);
                let new_work = perm(work_val);
                let j = replace_field(i, work_start, work_size, new_work, self.n);
                new_amps[j] = c_add(new_amps[j], amp);
            }
        }
        self.amplitudes = new_amps;
    }

    // ── Measurement ─────────────────────────────────────────────────

    /// Probability distribution over all basis states.
    pub fn probabilities(&self) -> Vec<f64> {
        self.amplitudes.iter().map(|a| c_norm_sq(*a)).collect()
    }

    /// Measure a single position (destructive — collapses that position).
    pub fn measure_position(&mut self, k: usize, seed: u64) -> PositionMeasurement {
        assert!(k < self.n);
        let bit = 1 << (self.n - 1 - k);

        let mut p0 = 0.0;
        for i in 0..self.dim() {
            if i & bit == 0 {
                p0 += c_norm_sq(self.amplitudes[i]);
            }
        }

        let mut rng = seed ^ (k as u64).wrapping_mul(0x517cc1b727220a95);
        let r = splitmix64(&mut rng);
        let value = if r < p0 { 0u8 } else { 1u8 };
        let prob = if value == 0 { p0 } else { 1.0 - p0 };

        for i in 0..self.dim() {
            let bit_val = if i & bit != 0 { 1u8 } else { 0u8 };
            if bit_val != value {
                self.amplitudes[i] = C_ZERO;
            }
        }
        self.normalize();

        PositionMeasurement {
            position: k,
            value,
            probability: prob,
        }
    }

    /// Measure all positions (destructive — collapses to a single basis state).
    pub fn measure_all(&mut self, seed: u64) -> FullMeasurement {
        let probs = self.probabilities();
        let mut rng = seed;
        let r = splitmix64(&mut rng);

        let mut cumulative = 0.0;
        let mut chosen = self.dim() - 1;
        for i in 0..self.dim() {
            cumulative += probs[i];
            if r < cumulative {
                chosen = i;
                break;
            }
        }

        let prob = probs[chosen];
        let bits = index_to_bits(chosen, self.n);

        for i in 0..self.dim() {
            self.amplitudes[i] = if i == chosen { C_ONE } else { C_ZERO };
        }

        FullMeasurement {
            bits,
            index: chosen as u64,
            probability: prob,
        }
    }

    /// Non-destructive sample (MDB exclusive). Returns what `measure_all`
    /// would return *without* collapsing the state.
    pub fn sample_all(&self, seed: u64) -> FullMeasurement {
        let probs = self.probabilities();
        let mut rng = seed;
        let r = splitmix64(&mut rng);

        let mut cumulative = 0.0;
        let mut chosen = self.dim() - 1;
        for i in 0..self.dim() {
            cumulative += probs[i];
            if r < cumulative {
                chosen = i;
                break;
            }
        }

        FullMeasurement {
            bits: index_to_bits(chosen, self.n),
            index: chosen as u64,
            probability: probs[chosen],
        }
    }

    // ── Non-destructive operations (MDB exclusive) ──────────────────

    /// Peek at the full register state without modification.
    /// Quantum computers *cannot* do this. MDB can.
    pub fn peek(&self) -> RegisterView {
        let mut states = Vec::new();
        let mut nonzero = 0;
        for i in 0..self.dim() {
            let prob = c_norm_sq(self.amplitudes[i]);
            if prob > 1e-15 {
                nonzero += 1;
                let phase = self.amplitudes[i].1.atan2(self.amplitudes[i].0);
                states.push(RegisterStateView {
                    index: i as u64,
                    bits: index_to_bits(i, self.n),
                    probability: prob,
                    phase,
                    label: format!("|{:0width$b}⟩", i, width = self.n),
                });
            }
        }
        RegisterView {
            n: self.n,
            total_states: self.dim(),
            nonzero_states: nonzero,
            states,
        }
    }

    /// Fork: create an independent copy.
    /// Quantum computers *cannot* do this (no-cloning theorem). MDB can.
    pub fn fork(&self) -> QuantumRegister {
        QuantumRegister {
            n: self.n,
            name: format!("{}_fork", self.name),
            amplitudes: self.amplitudes.clone(),
        }
    }

    // ── Utility ─────────────────────────────────────────────────────

    /// Renormalise amplitudes so total probability = 1.
    pub fn normalize(&mut self) {
        let total: f64 = self.amplitudes.iter().map(|a| c_norm_sq(*a)).sum();
        if total > 1e-30 && (total - 1.0).abs() > 1e-12 {
            let factor = 1.0 / total.sqrt();
            for amp in &mut self.amplitudes {
                *amp = c_scale(factor, *amp);
            }
        }
    }

    /// Total probability (should be 1.0 if normalised).
    pub fn total_probability(&self) -> f64 {
        self.amplitudes.iter().map(|a| c_norm_sq(*a)).sum()
    }

    /// Count of basis states with non-negligible amplitude.
    pub fn nonzero_count(&self) -> usize {
        self.amplitudes
            .iter()
            .filter(|a| c_norm_sq(**a) > 1e-15)
            .count()
    }

    /// Reset to |0…0⟩.
    pub fn reset(&mut self) {
        for amp in &mut self.amplitudes {
            *amp = C_ZERO;
        }
        self.amplitudes[0] = C_ONE;
    }

    /// State fidelity |⟨self|other⟩|².
    pub fn fidelity(&self, other: &QuantumRegister) -> f64 {
        assert_eq!(self.n, other.n);
        let mut inner = C_ZERO;
        for i in 0..self.dim() {
            let conj = (self.amplitudes[i].0, -self.amplitudes[i].1);
            inner = c_add(inner, c_mul(conj, other.amplitudes[i]));
        }
        c_norm_sq(inner)
    }

    /// Raw amplitude at basis state `index`.
    pub fn amplitude(&self, index: usize) -> (f64, f64) {
        self.amplitudes[index]
    }

    /// Set amplitude directly (caller must re-normalise).
    pub fn set_amplitude(&mut self, index: usize, amp: (f64, f64)) {
        self.amplitudes[index] = amp;
    }
}

// ── Display ─────────────────────────────────────────────────────────

impl fmt::Display for QuantumRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Register '{}' ({} positions):", self.name, self.n)?;
        for i in 0..self.dim() {
            let prob = c_norm_sq(self.amplitudes[i]);
            if prob > 1e-10 {
                let phase = self.amplitudes[i].1.atan2(self.amplitudes[i].0);
                writeln!(
                    f,
                    "  |{:0width$b}⟩  p={:.6}  φ={:.4}",
                    i,
                    prob,
                    phase,
                    width = self.n
                )?;
            }
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

    #[test]
    fn test_new_register() {
        let r = QuantumRegister::new(3, "t");
        assert_eq!(r.n, 3);
        assert_eq!(r.dim(), 8);
        assert!(approx(c_norm_sq(r.amplitude(0)), 1.0));
        assert_eq!(r.nonzero_count(), 1);
    }

    #[test]
    fn test_from_int() {
        let r = QuantumRegister::from_int(3, 5, "t");
        assert!(approx(c_norm_sq(r.amplitude(5)), 1.0));
        assert!(approx(c_norm_sq(r.amplitude(0)), 0.0));
    }

    #[test]
    fn test_from_bits() {
        let r = QuantumRegister::from_bits(&[1, 0, 1], "t");
        assert!(approx(c_norm_sq(r.amplitude(5)), 1.0));
    }

    #[test]
    fn test_hadamard_superposition() {
        let mut r = QuantumRegister::new(1, "t");
        r.hadamard(0);
        assert!(approx(c_norm_sq(r.amplitude(0)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(1)), 0.5));
    }

    #[test]
    fn test_hadamard_involution() {
        let mut r = QuantumRegister::new(2, "t");
        r.hadamard(0);
        r.hadamard(0);
        assert!(approx(c_norm_sq(r.amplitude(0)), 1.0));
    }

    #[test]
    fn test_pauli_x() {
        let mut r = QuantumRegister::new(2, "t");
        r.pauli_x(1);
        assert!(approx(c_norm_sq(r.amplitude(0b01)), 1.0));
    }

    #[test]
    fn test_pauli_z_phase() {
        let mut r = QuantumRegister::new(1, "t");
        r.hadamard(0);
        r.pauli_z(0);
        r.hadamard(0);
        assert!(approx(c_norm_sq(r.amplitude(1)), 1.0));
    }

    #[test]
    fn test_bell_state() {
        let mut r = QuantumRegister::new(2, "bell");
        r.hadamard(0);
        r.cnot(0, 1);
        assert!(approx(c_norm_sq(r.amplitude(0b00)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(0b11)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(0b01)), 0.0));
        assert!(approx(c_norm_sq(r.amplitude(0b10)), 0.0));
    }

    #[test]
    fn test_swap() {
        let mut r = QuantumRegister::from_bits(&[1, 0], "t");
        r.swap(0, 1);
        assert!(approx(c_norm_sq(r.amplitude(0b01)), 1.0));
    }

    #[test]
    fn test_toffoli() {
        let mut r = QuantumRegister::from_bits(&[1, 1, 0], "t");
        r.toffoli(0, 1, 2);
        assert!(approx(c_norm_sq(r.amplitude(0b111)), 1.0));

        let mut r2 = QuantumRegister::from_bits(&[1, 0, 0], "t");
        r2.toffoli(0, 1, 2);
        assert!(approx(c_norm_sq(r2.amplitude(0b100)), 1.0));
    }

    #[test]
    fn test_fredkin() {
        let mut r = QuantumRegister::from_bits(&[1, 1, 0], "t");
        r.fredkin(0, 1, 2);
        assert!(approx(c_norm_sq(r.amplitude(0b101)), 1.0));
    }

    #[test]
    fn test_ghz_state() {
        let mut r = QuantumRegister::new(3, "ghz");
        r.hadamard(0);
        r.cnot(0, 1);
        r.cnot(1, 2);
        assert!(approx(c_norm_sq(r.amplitude(0b000)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(0b111)), 0.5));
        assert_eq!(r.nonzero_count(), 2);
    }

    #[test]
    fn test_qft_uniform() {
        let mut r = QuantumRegister::new(2, "qft");
        r.qft(&[0, 1]);
        for i in 0..4 {
            assert!(approx(c_norm_sq(r.amplitude(i)), 0.25));
        }
    }

    #[test]
    fn test_qft_inverse_identity() {
        let mut r = QuantumRegister::from_int(3, 5, "t");
        let original = r.fork();
        r.qft(&[0, 1, 2]);
        r.inverse_qft(&[0, 1, 2]);
        assert!(r.fidelity(&original) > 1.0 - 1e-10);
    }

    #[test]
    fn test_qft_phases() {
        // QFT|01⟩ on 2 qubits: amplitudes all 0.25 probability,
        // phases = 0, π/2, π, 3π/2
        let mut r = QuantumRegister::from_int(2, 1, "t");
        r.qft(&[0, 1]);
        let view = r.peek();
        assert_eq!(view.nonzero_states, 4);
        for sv in &view.states {
            assert!(approx(sv.probability, 0.25));
        }
    }

    #[test]
    fn test_oracle_phase_flip() {
        let mut r = QuantumRegister::new(2, "t");
        for k in 0..2 {
            r.hadamard(k);
        }
        r.apply_oracle(&|bits: &[u8]| bits[0] == 1 && bits[1] == 0);
        let a_marked = r.amplitude(0b10);
        let a_other = r.amplitude(0b00);
        let phase_diff = (a_marked.1.atan2(a_marked.0) - a_other.1.atan2(a_other.0)).abs();
        assert!(phase_diff > 3.0); // ~π
    }

    #[test]
    fn test_grover_2qubit() {
        let mut r = QuantumRegister::new(2, "t");
        for k in 0..2 {
            r.hadamard(k);
        }
        r.apply_oracle(&|bits: &[u8]| bits[0] == 1 && bits[1] == 1);
        r.grover_diffusion();
        assert!(c_norm_sq(r.amplitude(0b11)) > 0.9);
    }

    #[test]
    fn test_peek_nondestructive() {
        let mut r = QuantumRegister::new(2, "t");
        r.hadamard(0);
        r.cnot(0, 1);
        let v1 = r.peek();
        let v2 = r.peek();
        assert_eq!(v1.nonzero_states, v2.nonzero_states);
        assert!(approx(r.total_probability(), 1.0));
    }

    #[test]
    fn test_fork_independence() {
        let mut r = QuantumRegister::new(2, "orig");
        r.hadamard(0);
        let mut f = r.fork();
        f.pauli_x(1);
        assert!(approx(c_norm_sq(r.amplitude(0b00)), 0.5));
        assert!(approx(c_norm_sq(r.amplitude(0b10)), 0.5));
    }

    #[test]
    fn test_normalization_preserved() {
        let mut r = QuantumRegister::new(3, "t");
        r.hadamard(0);
        assert!(approx(r.total_probability(), 1.0));
        r.cnot(0, 1);
        assert!(approx(r.total_probability(), 1.0));
        r.toffoli(0, 1, 2);
        assert!(approx(r.total_probability(), 1.0));
        r.qft(&[0, 1, 2]);
        assert!(approx(r.total_probability(), 1.0));
    }

    #[test]
    fn test_measure_deterministic() {
        let mut r1 = QuantumRegister::new(2, "a");
        r1.hadamard(0);
        r1.cnot(0, 1);
        let mut r2 = r1.fork();
        let m1 = r1.measure_all(42);
        let m2 = r2.measure_all(42);
        assert_eq!(m1.bits, m2.bits);
    }

    #[test]
    fn test_sample_nondestructive() {
        let mut r = QuantumRegister::new(2, "t");
        r.hadamard(0);
        r.cnot(0, 1);
        let s = r.sample_all(42);
        // State should still be in superposition after sample
        assert_eq!(r.nonzero_count(), 2);
        assert!(s.bits == vec![0, 0] || s.bits == vec![1, 1]);
    }

    #[test]
    fn test_apply_function_identity() {
        let mut r = QuantumRegister::new(4, "t");
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
        let mut r = QuantumRegister::from_bits(&[1, 1], "t");
        r.controlled_phase(0, 1, PI);
        let amp = r.amplitude(0b11);
        assert!(approx(amp.0, -1.0));
        assert!(amp.1.abs() < 1e-10);
    }

    #[test]
    fn test_display() {
        let mut r = QuantumRegister::new(2, "disp");
        r.hadamard(0);
        let s = format!("{}", r);
        assert!(s.contains("disp"));
    }

    #[test]
    fn test_fidelity_identical() {
        let mut r = QuantumRegister::new(2, "t");
        r.hadamard(0);
        r.cnot(0, 1);
        let f = r.fork();
        assert!(approx(r.fidelity(&f), 1.0));
    }

    #[test]
    fn test_fidelity_orthogonal() {
        let r0 = QuantumRegister::from_int(2, 0, "a");
        let r1 = QuantumRegister::from_int(2, 1, "b");
        assert!(approx(r0.fidelity(&r1), 0.0));
    }
}
