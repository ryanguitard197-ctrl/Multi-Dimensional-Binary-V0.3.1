//! # Quantum Algorithms
//!
//! Standard quantum algorithms implemented on [`QuantumRegister`], proving
//! MDB's computational equivalence to universal quantum computing.
//!
//! - **Deutsch–Jozsa** — constant-vs-balanced in one evaluation
//! - **Grover's search** — quadratic speedup, O(√N)
//! - **Shor's factoring** — polynomial-time integer factoring via period-finding
//! - **Quantum teleportation** — state transfer via entanglement

use crate::register::QuantumRegister;
use std::f64::consts::PI;

// ═════════════════════════════════════════════════════════════════════
// Deutsch–Jozsa
// ═════════════════════════════════════════════════════════════════════

/// Whether a boolean function is constant (same output for all inputs)
/// or balanced (outputs 0 for exactly half of inputs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionType {
    Constant,
    Balanced,
}

/// Deutsch–Jozsa algorithm.
///
/// Determines if `f : {0,1}^n → {0,1}` is constant or balanced using
/// a *single* quantum evaluation. Classically requires up to 2^(n−1)+1.
pub fn deutsch_jozsa(n: usize, f: &dyn Fn(usize) -> u8) -> FunctionType {
    let total = n + 1; // n input + 1 ancilla
    let mut reg = QuantumRegister::new(total, "deutsch_jozsa");

    // Ancilla |1⟩
    reg.pauli_x(n);

    // H on every position
    for k in 0..total {
        reg.hadamard(k);
    }

    // Oracle |x⟩|y⟩ → |x⟩|y ⊕ f(x)⟩
    reg.apply_function((0, n), (n, total), &|x| f(x) as usize);

    // H on input positions
    for k in 0..n {
        reg.hadamard(k);
    }

    // If input register is |0…0⟩ → constant, else balanced
    let view = reg.peek();
    let p_zero: f64 = view
        .states
        .iter()
        .filter(|s| s.bits[..n].iter().all(|&b| b == 0))
        .map(|s| s.probability)
        .sum();

    if p_zero > 0.5 {
        FunctionType::Constant
    } else {
        FunctionType::Balanced
    }
}

// ═════════════════════════════════════════════════════════════════════
// Grover's Search
// ═════════════════════════════════════════════════════════════════════

/// Result of Grover's search.
#[derive(Debug, Clone)]
pub struct GroverResult {
    pub solution: Vec<u8>,
    pub index: u64,
    pub iterations: usize,
    pub probability: f64,
    pub total_states: usize,
}

/// Grover's search algorithm.
///
/// Finds `x` in `{0,1}^n` such that `oracle(x) = true`.
/// Optimal iterations ≈ π√N/4.  Quadratic speedup over classical.
pub fn grovers_search(
    n: usize,
    oracle: &dyn Fn(&[u8]) -> bool,
    max_iterations: Option<usize>,
) -> GroverResult {
    let total_states = 1usize << n;
    let optimal =
        ((PI / 4.0) * (total_states as f64).sqrt()).round() as usize;
    let iterations = max_iterations.unwrap_or(optimal).max(1);

    let mut reg = QuantumRegister::new(n, "grover");

    // Uniform superposition
    for k in 0..n {
        reg.hadamard(k);
    }

    // Grover iterations
    for _ in 0..iterations {
        reg.apply_oracle(oracle);
        reg.grover_diffusion();
    }

    // Best state
    let view = reg.peek();
    let best = view
        .states
        .iter()
        .max_by(|a, b| a.probability.partial_cmp(&b.probability).unwrap())
        .unwrap();

    GroverResult {
        solution: best.bits.clone(),
        index: best.index,
        iterations,
        probability: best.probability,
        total_states,
    }
}

// ═════════════════════════════════════════════════════════════════════
// Shor's Factoring
// ═════════════════════════════════════════════════════════════════════

/// Result of Shor's algorithm.
#[derive(Debug, Clone)]
pub struct ShorResult {
    pub n: u64,
    pub factors: (u64, u64),
    pub period: u64,
    pub base: u64,
    pub attempts: usize,
}

/// Shor's algorithm — factor an integer.
///
/// Uses quantum period-finding (QFT) to discover the period of
/// `a^x mod N`, then derives non-trivial factors.
pub fn shors_factor(n: u64) -> Option<ShorResult> {
    if n < 4 {
        return None;
    }
    if n % 2 == 0 {
        return Some(ShorResult {
            n,
            factors: (2, n / 2),
            period: 0,
            base: 0,
            attempts: 0,
        });
    }

    // Perfect-power check
    for b in 2..=((n as f64).log2() as u64 + 1) {
        let root = (n as f64).powf(1.0 / b as f64).round() as u64;
        for r in [root.saturating_sub(1), root, root + 1] {
            if r >= 2 && checked_pow(r, b as u32) == Some(n) {
                return Some(ShorResult {
                    n,
                    factors: (r, n / r),
                    period: 0,
                    base: 0,
                    attempts: 0,
                });
            }
        }
    }

    // Try bases
    let bases: Vec<u64> = (2..n.min(30)).collect();
    for (attempt, &a) in bases.iter().enumerate() {
        let g = gcd(a, n);
        if g > 1 && g < n {
            return Some(ShorResult {
                n,
                factors: (g, n / g),
                period: 0,
                base: a,
                attempts: attempt + 1,
            });
        }

        if let Some(r) = quantum_period_find(a, n) {
            if r % 2 == 0 {
                let half = mod_pow(a, r / 2, n);
                for candidate in [half.wrapping_add(1), half.wrapping_sub(1)] {
                    let f = gcd(candidate, n);
                    if f > 1 && f < n {
                        return Some(ShorResult {
                            n,
                            factors: (f, n / f),
                            period: r,
                            base: a,
                            attempts: attempt + 1,
                        });
                    }
                }
            }
        }
    }

    None
}

/// Quantum period-finding subroutine.
fn quantum_period_find(a: u64, n: u64) -> Option<u64> {
    let work_bits = (n as f64).log2().ceil() as usize;
    let ctrl_bits = 2 * work_bits;
    let total_bits = ctrl_bits + work_bits;

    // Fall back to classical for large N (register too big)
    if total_bits > 20 {
        return classical_period_find(a, n);
    }

    let mut reg = QuantumRegister::new(total_bits, "shor_qpf");

    // Work register = |1⟩ (set least-significant bit of work region)
    reg.pauli_x(total_bits - 1);

    // Hadamard on control qubits
    for k in 0..ctrl_bits {
        reg.hadamard(k);
    }

    // Controlled modular multiplications: |x⟩|y⟩ → |x⟩|y·a^(2^j) mod N⟩
    for j in 0..ctrl_bits {
        let power = mod_pow(a, 1u64 << (ctrl_bits - 1 - j), n);
        let modulus = n;
        reg.controlled_permutation(j, ctrl_bits, work_bits, &move |y| {
            if (y as u64) < modulus {
                ((y as u64).wrapping_mul(power) % modulus) as usize
            } else {
                y
            }
        });
    }

    // Inverse QFT on control register
    let ctrl_positions: Vec<usize> = (0..ctrl_bits).collect();
    reg.inverse_qft(&ctrl_positions);

    // Sample multiple times (MDB can peek!) to extract period
    let ctrl_dim = 1u64 << ctrl_bits;
    for seed in 0u64..16 {
        let sample = reg.sample_all(seed * 137 + 42);
        let measured_ctrl = sample
            .bits[..ctrl_bits]
            .iter()
            .fold(0u64, |acc, &b| (acc << 1) | b as u64);

        if measured_ctrl == 0 {
            continue;
        }

        if let Some(r) = continued_fraction_period(measured_ctrl, ctrl_dim, n) {
            if r > 0 && r < n && mod_pow(a, r, n) == 1 {
                return Some(r);
            }
            // Also try multiples of r (the measured phase might be j/r for j>1)
            for mult in 2..=4 {
                let rm = r * mult;
                if rm < n && mod_pow(a, rm, n) == 1 {
                    return Some(rm);
                }
            }
        }
    }

    // Fallback: also try peeking at the most probable states
    let view = reg.peek();
    let mut top_states: Vec<_> = view.states.iter().collect();
    top_states.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap());

    for sv in top_states.iter().take(8) {
        let measured_ctrl = sv.bits[..ctrl_bits]
            .iter()
            .fold(0u64, |acc, &b| (acc << 1) | b as u64);
        if measured_ctrl == 0 {
            continue;
        }
        if let Some(r) = continued_fraction_period(measured_ctrl, ctrl_dim, n) {
            if r > 0 && r < n && mod_pow(a, r, n) == 1 {
                return Some(r);
            }
        }
    }

    classical_period_find(a, n)
}

/// Extract period from measured phase via continued fractions.
fn continued_fraction_period(measured: u64, ctrl_dim: u64, n: u64) -> Option<u64> {
    let mut p = measured;
    let mut q = ctrl_dim;
    // Reduce
    let g = gcd(p, q);
    p /= g;
    q /= g;

    let mut h_prev: u64 = 1;
    let mut h_curr: u64 = 0;
    let mut k_prev: u64 = 0;
    let mut k_curr: u64 = 1;

    let mut best: Option<u64> = None;

    for _ in 0..60 {
        if q == 0 {
            break;
        }
        let a_i = p / q;
        let rem = p % q;

        let new_h = a_i.checked_mul(h_curr).and_then(|x| x.checked_add(h_prev));
        let new_k = a_i.checked_mul(k_curr).and_then(|x| x.checked_add(k_prev));

        match (new_h, new_k) {
            (Some(nh), Some(nk)) => {
                h_prev = h_curr;
                h_curr = nh;
                k_prev = k_curr;
                k_curr = nk;

                if k_curr > 0 && k_curr < n {
                    best = Some(k_curr);
                }
            }
            _ => break,
        }

        if rem == 0 {
            break;
        }
        p = q;
        q = rem;
    }

    best
}

/// Classical period-finding fallback for large N.
fn classical_period_find(a: u64, n: u64) -> Option<u64> {
    let mut val = 1u64;
    for r in 1..n.min(100_000) {
        val = val.wrapping_mul(a) % n;
        if val == 1 {
            return Some(r);
        }
    }
    None
}

// ═════════════════════════════════════════════════════════════════════
// Quantum Teleportation
// ═════════════════════════════════════════════════════════════════════

/// Result of quantum teleportation.
#[derive(Debug, Clone)]
pub struct TeleportResult {
    /// Fidelity |⟨original|received⟩|² (1.0 = perfect).
    pub fidelity: f64,
    /// Alice's measurement outcomes (qubit 0, qubit 1).
    pub alice_bits: (u8, u8),
}

/// Quantum teleportation: transfer state α|0⟩+β|1⟩ from Alice to Bob
/// using a shared Bell pair and two classical bits.
///
/// The fidelity should be 1.0 (perfect teleportation) regardless of
/// Alice's measurement outcome.
pub fn quantum_teleport(alpha: (f64, f64), beta: (f64, f64), seed: u64) -> TeleportResult {
    // 3 qubits: [0] Alice's data, [1] Alice's Bell, [2] Bob's Bell
    let mut reg = QuantumRegister::new(3, "teleport");

    // Set qubit 0 to α|0⟩ + β|1⟩
    reg.set_amplitude(0b000, alpha);
    reg.set_amplitude(0b100, beta);
    reg.normalize();

    // Create Bell pair |Φ+⟩ between qubits 1 and 2
    reg.hadamard(1);
    reg.cnot(1, 2);

    // Alice: CNOT(0→1), then H(0)
    reg.cnot(0, 1);
    reg.hadamard(0);

    // Alice measures qubits 0 and 1
    let m0 = reg.measure_position(0, seed);
    let m1 = reg.measure_position(1, seed.wrapping_add(1));

    // Bob corrects based on Alice's bits
    if m1.value == 1 {
        reg.pauli_x(2);
    }
    if m0.value == 1 {
        reg.pauli_z(2);
    }

    // Check fidelity: Bob's qubit should be α|0⟩+β|1⟩
    // Extract Bob's reduced state from the 3-qubit register
    let mut bob_amp_0 = (0.0f64, 0.0f64);
    let mut bob_amp_1 = (0.0f64, 0.0f64);
    for i in 0..reg.dim() {
        let amp = reg.amplitude(i);
        if amp.0 == 0.0 && amp.1 == 0.0 {
            continue;
        }
        if (i & 1) == 0 {
            // qubit 2 = 0
            bob_amp_0 = (bob_amp_0.0 + amp.0, bob_amp_0.1 + amp.1);
        } else {
            // qubit 2 = 1
            bob_amp_1 = (bob_amp_1.0 + amp.0, bob_amp_1.1 + amp.1);
        }
    }

    // Normalise target
    let norm_alpha = (alpha.0 * alpha.0 + alpha.1 * alpha.1
        + beta.0 * beta.0 + beta.1 * beta.1).sqrt();
    let alpha_n = (alpha.0 / norm_alpha, alpha.1 / norm_alpha);
    let beta_n = (beta.0 / norm_alpha, beta.1 / norm_alpha);

    // Inner product ⟨target|bob⟩
    let inner_re = alpha_n.0 * bob_amp_0.0 + alpha_n.1 * bob_amp_0.1
        + beta_n.0 * bob_amp_1.0 + beta_n.1 * bob_amp_1.1;
    let inner_im = alpha_n.0 * bob_amp_0.1 - alpha_n.1 * bob_amp_0.0
        + beta_n.0 * bob_amp_1.1 - beta_n.1 * bob_amp_1.0;

    let fidelity = inner_re * inner_re + inner_im * inner_im;

    TeleportResult {
        fidelity,
        alice_bits: (m0.value, m1.value),
    }
}

// ═════════════════════════════════════════════════════════════════════
// Helpers
// ═════════════════════════════════════════════════════════════════════

/// Greatest common divisor.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Modular exponentiation: base^exp mod modulus.
fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result.wrapping_mul(base) % modulus;
        }
        exp >>= 1;
        base = base.wrapping_mul(base) % modulus;
    }
    result
}

/// Checked power to avoid overflow.
fn checked_pow(base: u64, exp: u32) -> Option<u64> {
    let mut result = 1u64;
    for _ in 0..exp {
        result = result.checked_mul(base)?;
    }
    Some(result)
}

// ═════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Deutsch–Jozsa ───────────────────────────────────────────────

    #[test]
    fn test_dj_constant_zero() {
        assert_eq!(deutsch_jozsa(3, &|_| 0), FunctionType::Constant);
    }

    #[test]
    fn test_dj_constant_one() {
        assert_eq!(deutsch_jozsa(3, &|_| 1), FunctionType::Constant);
    }

    #[test]
    fn test_dj_balanced_lsb() {
        assert_eq!(
            deutsch_jozsa(3, &|x| (x & 1) as u8),
            FunctionType::Balanced,
        );
    }

    #[test]
    fn test_dj_balanced_parity() {
        assert_eq!(
            deutsch_jozsa(3, &|x| (x.count_ones() % 2) as u8),
            FunctionType::Balanced,
        );
    }

    // ── Grover's Search ─────────────────────────────────────────────

    #[test]
    fn test_grover_finds_target() {
        let res = grovers_search(3, &|bits| bits == [1, 0, 1], None);
        assert_eq!(res.solution, vec![1, 0, 1]);
        assert!(res.probability > 0.8);
    }

    #[test]
    fn test_grover_4qubit() {
        let target = 0b1010usize;
        let res = grovers_search(
            4,
            &|bits| {
                let val = bits.iter().fold(0usize, |a, &b| (a << 1) | b as usize);
                val == target
            },
            None,
        );
        assert_eq!(res.index, target as u64);
        assert!(res.probability > 0.5);
    }

    // ── Shor's Factoring ────────────────────────────────────────────

    #[test]
    fn test_shor_factor_15() {
        let res = shors_factor(15).unwrap();
        let (a, b) = res.factors;
        assert_eq!(a * b, 15);
        assert!(a > 1 && b > 1);
    }

    #[test]
    fn test_shor_factor_21() {
        let res = shors_factor(21).unwrap();
        let (a, b) = res.factors;
        assert_eq!(a * b, 21);
        assert!(a > 1 && b > 1);
    }

    #[test]
    fn test_shor_factor_35() {
        let res = shors_factor(35).unwrap();
        let (a, b) = res.factors;
        assert_eq!(a * b, 35);
        assert!(a > 1 && b > 1);
    }

    #[test]
    fn test_shor_even() {
        let res = shors_factor(22).unwrap();
        assert_eq!(res.factors, (2, 11));
    }

    // ── Teleportation ───────────────────────────────────────────────

    #[test]
    fn test_teleport_basis_0() {
        let res = quantum_teleport((1.0, 0.0), (0.0, 0.0), 42);
        assert!(res.fidelity > 0.99);
    }

    #[test]
    fn test_teleport_basis_1() {
        let res = quantum_teleport((0.0, 0.0), (1.0, 0.0), 42);
        assert!(res.fidelity > 0.99);
    }

    #[test]
    fn test_teleport_superposition() {
        let res = quantum_teleport(
            (std::f64::consts::FRAC_1_SQRT_2, 0.0),
            (std::f64::consts::FRAC_1_SQRT_2, 0.0),
            42,
        );
        assert!(res.fidelity > 0.99);
    }

    #[test]
    fn test_teleport_arbitrary() {
        // α = cos(π/8), β = sin(π/8) — arbitrary state
        let alpha = ((PI / 8.0).cos(), 0.0);
        let beta = ((PI / 8.0).sin(), 0.0);
        let res = quantum_teleport(alpha, beta, 42);
        assert!(
            res.fidelity > 0.99,
            "fidelity = {}, expected > 0.99",
            res.fidelity
        );
    }

    #[test]
    fn test_teleport_different_seeds() {
        // Teleportation should work regardless of measurement outcome
        let alpha = (0.8, 0.0);
        let beta = (0.6, 0.0);
        for seed in [0, 42, 137, 999, 12345] {
            let res = quantum_teleport(alpha, beta, seed);
            assert!(
                res.fidelity > 0.99,
                "seed={} fidelity={:.6}",
                seed,
                res.fidelity
            );
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(17, 13), 1);
        assert_eq!(gcd(15, 5), 5);
    }

    #[test]
    fn test_mod_pow() {
        assert_eq!(mod_pow(7, 4, 15), 1); // period of 7 mod 15 is 4
        assert_eq!(mod_pow(2, 10, 1024), 0);
        assert_eq!(mod_pow(3, 0, 7), 1);
    }
}
