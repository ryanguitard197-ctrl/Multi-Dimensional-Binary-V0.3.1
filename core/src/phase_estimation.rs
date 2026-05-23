//! # Quantum Phase Estimation (QPE)
//!
//! Estimates the eigenvalue phase of a unitary operator.
//! Given U|ψ⟩ = e^(2πiθ)|ψ⟩, QPE extracts θ to `t` bits of precision.
//!
//! This is a key subroutine for:
//! - Shor's algorithm (period finding)
//! - Quantum chemistry (molecular energy levels)
//! - HHL algorithm (linear systems)

use crate::register::QuantumRegister;
use std::f64::consts::PI;

/// Result of phase estimation.
#[derive(Debug, Clone)]
pub struct PhaseEstimationResult {
    /// Estimated phase θ ∈ [0, 1).
    pub phase: f64,
    /// Number of precision bits used.
    pub precision_bits: usize,
    /// Raw measurement from the counting register.
    pub raw_measurement: usize,
    /// Probability of the measured state.
    pub probability: f64,
    /// All detected phases with probabilities (MDB exclusive — via peek).
    pub all_phases: Vec<(f64, f64)>,
}

/// Quantum Phase Estimation.
///
/// Estimates the eigenvalue phase θ of a unitary U, where U|ψ⟩ = e^(2πiθ)|ψ⟩.
///
/// The unitary is specified as a function that applies U^(2^k) to the
/// target register, parameterised by k (the power).
///
/// # Arguments
/// - `precision_bits` — Number of counting qubits (accuracy ~ 1/2^t)
/// - `target_bits` — Number of qubits in the eigenstate register
/// - `prepare_eigenstate` — Prepares |ψ⟩ in the target register
/// - `apply_controlled_u_power` — Applies controlled-U^(2^k) with control qubit at position `ctrl` and target qubits starting at `target_start`
pub fn phase_estimation(
    precision_bits: usize,
    target_bits: usize,
    prepare_eigenstate: &dyn Fn(&mut QuantumRegister),
    apply_controlled_u_power: &dyn Fn(&mut QuantumRegister, usize, usize, usize),
) -> PhaseEstimationResult {
    let total = precision_bits + target_bits;
    assert!(total <= 24, "total qubits must be ≤ 24");

    let mut reg = QuantumRegister::new(total, "qpe");

    // Prepare eigenstate in target register
    prepare_eigenstate(&mut reg);

    // Hadamard on all counting qubits
    for k in 0..precision_bits {
        reg.hadamard(k);
    }

    // Controlled-U^(2^k) for each counting qubit
    for k in 0..precision_bits {
        let power = precision_bits - 1 - k; // MSB first
        apply_controlled_u_power(&mut reg, k, precision_bits, power);
    }

    // Inverse QFT on counting register
    let positions: Vec<usize> = (0..precision_bits).collect();
    reg.inverse_qft(&positions);

    // Extract phases via peek (MDB exclusive)
    let view = reg.peek();

    let mut phase_probs: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    for sv in &view.states {
        let counting_val = sv.bits[..precision_bits]
            .iter()
            .fold(0usize, |acc, &b| (acc << 1) | b as usize);
        *phase_probs.entry(counting_val).or_insert(0.0) += sv.probability;
    }

    let mut all_phases: Vec<(f64, f64)> = phase_probs
        .iter()
        .map(|(&val, &prob)| {
            let phase = val as f64 / (1u64 << precision_bits) as f64;
            (phase, prob)
        })
        .filter(|(_, p)| *p > 1e-10)
        .collect();
    all_phases.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let (best_phase, best_prob) = all_phases.first().cloned().unwrap_or((0.0, 0.0));
    let best_raw = (best_phase * (1u64 << precision_bits) as f64).round() as usize;

    PhaseEstimationResult {
        phase: best_phase,
        precision_bits,
        raw_measurement: best_raw,
        probability: best_prob,
        all_phases,
    }
}

/// Simplified QPE for a phase-gate unitary.
///
/// The unitary is U = Phase(2πθ), so U|1⟩ = e^(2πiθ)|1⟩.
/// This is the simplest case for testing QPE.
pub fn estimate_phase_gate(theta: f64, precision_bits: usize) -> PhaseEstimationResult {
    phase_estimation(
        precision_bits,
        1, // single target qubit
        &|reg: &mut QuantumRegister| {
            // Prepare |1⟩ eigenstate
            let target_pos = reg.n - 1;
            reg.pauli_x(target_pos);
        },
        &move |reg: &mut QuantumRegister, ctrl: usize, target_start: usize, power: usize| {
            // Apply controlled-Phase(2π·θ·2^power)
            let angle = 2.0 * PI * theta * (1u64 << power) as f64;
            reg.controlled_phase(ctrl, target_start, angle);
        },
    )
}

/// Estimate eigenvalues of a 2×2 unitary matrix.
///
/// The unitary is specified as [[a, b], [c, d]] where each entry is (re, im).
/// Returns estimated eigenvalue phases.
pub fn estimate_eigenvalues_2x2(
    matrix: [[(f64, f64); 2]; 2],
    precision_bits: usize,
) -> Vec<PhaseEstimationResult> {
    // For a 2x2 unitary, we can estimate both eigenvalues by trying both eigenstates.
    // First, try with |0⟩ as target:
    let result_0 = phase_estimation(
        precision_bits,
        1,
        &|_reg: &mut QuantumRegister| {
            // |0⟩ is default
        },
        &move |reg: &mut QuantumRegister, ctrl: usize, target_start: usize, power: usize| {
            // Apply controlled-U^(2^power)
            // For simplicity, we apply U iteratively 2^power times
            let iterations = 1usize << power;
            for _ in 0..iterations.min(256) {
                // Apply controlled-U
                apply_controlled_2x2(reg, ctrl, target_start, matrix);
            }
        },
    );

    // Then with |1⟩:
    let result_1 = phase_estimation(
        precision_bits,
        1,
        &|reg: &mut QuantumRegister| {
            let target_pos = reg.n - 1;
            reg.pauli_x(target_pos);
        },
        &move |reg: &mut QuantumRegister, ctrl: usize, target_start: usize, power: usize| {
            let iterations = 1usize << power;
            for _ in 0..iterations.min(256) {
                apply_controlled_2x2(reg, ctrl, target_start, matrix);
            }
        },
    );

    vec![result_0, result_1]
}

/// Apply controlled-U where U is a 2×2 unitary matrix.
fn apply_controlled_2x2(
    reg: &mut QuantumRegister,
    ctrl: usize,
    target: usize,
    u: [[(f64, f64); 2]; 2],
) {
    // When control = |1⟩, apply U to target
    let c_bit = 1 << (reg.n - 1 - ctrl);
    let t_bit = 1 << (reg.n - 1 - target);
    let dim = reg.dim();

    let mut new_amps = Vec::with_capacity(dim);
    for i in 0..dim {
        new_amps.push(reg.amplitude(i));
    }

    for i in 0..dim {
        if i & c_bit == 0 {
            continue;
        }
        if i & t_bit != 0 {
            continue;
        }
        let j = i | t_bit;
        let a = reg.amplitude(i);
        let b = reg.amplitude(j);

        new_amps[i] = c_add(c_mul(u[0][0], a), c_mul(u[0][1], b));
        new_amps[j] = c_add(c_mul(u[1][0], a), c_mul(u[1][1], b));
    }

    for i in 0..dim {
        reg.set_amplitude(i, new_amps[i]);
    }
}

// Complex helpers (local to this module)
#[inline]
fn c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

#[inline]
fn c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

// ═════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_phase_estimation_quarter() {
        // θ = 0.25 → Phase(π/2) = S gate
        let result = estimate_phase_gate(0.25, 4);
        assert!(
            approx(result.phase, 0.25, 0.01),
            "expected 0.25, got {}",
            result.phase
        );
    }

    #[test]
    fn test_phase_estimation_half() {
        // θ = 0.5 → Phase(π) = Z gate
        let result = estimate_phase_gate(0.5, 4);
        assert!(
            approx(result.phase, 0.5, 0.01),
            "expected 0.5, got {}",
            result.phase
        );
    }

    #[test]
    fn test_phase_estimation_eighth() {
        // θ = 0.125 → Phase(π/4) = T gate
        let result = estimate_phase_gate(0.125, 4);
        assert!(
            approx(result.phase, 0.125, 0.01),
            "expected 0.125, got {}",
            result.phase
        );
    }

    #[test]
    fn test_phase_estimation_third() {
        // θ = 1/3 — not exactly representable in binary
        let result = estimate_phase_gate(1.0 / 3.0, 6);
        assert!(
            approx(result.phase, 1.0 / 3.0, 0.02),
            "expected ~0.333, got {}",
            result.phase
        );
    }

    #[test]
    fn test_phase_estimation_precision() {
        // More precision bits → better estimate
        let r4 = estimate_phase_gate(0.3, 4);
        let r8 = estimate_phase_gate(0.3, 8);
        let err4 = (r4.phase - 0.3).abs();
        let err8 = (r8.phase - 0.3).abs();
        assert!(
            err8 <= err4 + 0.01,
            "8-bit error ({}) should be ≤ 4-bit error ({})",
            err8,
            err4
        );
    }

    #[test]
    fn test_phase_estimation_zero() {
        // θ = 0 → identity
        let result = estimate_phase_gate(0.0, 4);
        assert!(
            approx(result.phase, 0.0, 0.01),
            "expected 0.0, got {}",
            result.phase
        );
    }

    #[test]
    fn test_peek_reveals_all_phases() {
        // For non-exact phases, peek should show multiple candidates
        let result = estimate_phase_gate(0.3, 4);
        assert!(
            result.all_phases.len() >= 1,
            "should detect at least 1 phase candidate"
        );
    }
}
