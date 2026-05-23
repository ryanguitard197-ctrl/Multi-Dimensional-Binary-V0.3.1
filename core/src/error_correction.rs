//! # Quantum Error Correction
//!
//! Error correction codes that detect and correct errors in quantum states.
//! These codes are essential for fault-tolerant quantum computing — and MDB
//! implements them all while also offering the MDB-exclusive ability to
//! *peek* at the state without measurement-induced collapse.
//!
//! ## Codes implemented:
//!
//! - **3-qubit bit-flip code** — Corrects single X errors
//! - **3-qubit phase-flip code** — Corrects single Z errors
//! - **Shor's 9-qubit code** — Corrects arbitrary single-qubit errors
//! - **Steane's 7-qubit code** — CSS code, corrects arbitrary single-qubit errors

use crate::register::QuantumRegister;

// ═════════════════════════════════════════════════════════════════════
// Bit-Flip Code (3-qubit)
// ═════════════════════════════════════════════════════════════════════

/// Encoded state for the 3-qubit bit-flip code.
///
/// Encodes α|0⟩+β|1⟩ as α|000⟩+β|111⟩.
/// Can correct a single X (bit-flip) error on any qubit.
pub struct BitFlipCode {
    pub register: QuantumRegister,
}

impl BitFlipCode {
    /// Encode α|0⟩+β|1⟩ into 3-qubit bit-flip code.
    pub fn encode(alpha: (f64, f64), beta: (f64, f64)) -> Self {
        let mut reg = QuantumRegister::new(3, "bit_flip");
        reg.set_amplitude(0b000, alpha);
        reg.set_amplitude(0b100, beta);
        reg.normalize();

        // |0⟩ → |000⟩, |1⟩ → |111⟩
        reg.cnot(0, 1);
        reg.cnot(0, 2);

        Self { register: reg }
    }

    /// Inject a bit-flip error on qubit `k`.
    pub fn inject_error(&mut self, k: usize) {
        assert!(k < 3);
        self.register.pauli_x(k);
    }

    /// Syndrome measurement + correction (non-destructive with MDB peek).
    ///
    /// Returns the syndrome bits and which qubit was corrected (if any).
    pub fn correct(&mut self) -> BitFlipSyndrome {
        // Syndrome extraction: compare qubit pairs
        // s1 = q0 ⊕ q1, s2 = q0 ⊕ q2
        // We use peek (MDB exclusive) to determine the syndrome
        let view = self.register.peek();

        // For each basis state with non-negligible amplitude, check consistency
        // In a proper state (possibly with one error), all amplitudes agree on syndrome
        let mut syndrome_counts = [0u32; 4]; // (s1, s2) pairs

        for sv in &view.states {
            let q0 = sv.bits[0];
            let q1 = sv.bits[1];
            let q2 = sv.bits[2];
            let s1 = q0 ^ q1;
            let s2 = q0 ^ q2;
            let idx = (s1 as usize) * 2 + s2 as usize;
            syndrome_counts[idx] += 1;
        }

        // Find dominant syndrome
        let max_idx = syndrome_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
            .unwrap_or(0);

        let s1 = (max_idx >> 1) as u8;
        let s2 = (max_idx & 1) as u8;

        let corrected_qubit = match (s1, s2) {
            (0, 0) => None,      // No error
            (1, 1) => {          // q0 flipped
                self.register.pauli_x(0);
                Some(0)
            }
            (1, 0) => {          // q1 flipped
                self.register.pauli_x(1);
                Some(1)
            }
            (0, 1) => {          // q2 flipped
                self.register.pauli_x(2);
                Some(2)
            }
            _ => None,
        };

        BitFlipSyndrome {
            s1,
            s2,
            corrected_qubit,
        }
    }

    /// Decode back to a single qubit's worth of information.
    /// Returns the probability of |0⟩ logical and |1⟩ logical.
    pub fn decode(&self) -> (f64, f64) {
        let view = self.register.peek();
        let mut p_zero = 0.0; // |000⟩
        let mut p_one = 0.0;  // |111⟩
        for sv in &view.states {
            if sv.bits == [0, 0, 0] {
                p_zero += sv.probability;
            } else if sv.bits == [1, 1, 1] {
                p_one += sv.probability;
            }
        }
        (p_zero, p_one)
    }
}

/// Syndrome for bit-flip code.
pub struct BitFlipSyndrome {
    pub s1: u8,
    pub s2: u8,
    pub corrected_qubit: Option<usize>,
}

// ═════════════════════════════════════════════════════════════════════
// Phase-Flip Code (3-qubit)
// ═════════════════════════════════════════════════════════════════════

/// Encoded state for the 3-qubit phase-flip code.
///
/// Encodes α|0⟩+β|1⟩ as α|+++⟩+β|---⟩.
/// Can correct a single Z (phase-flip) error on any qubit.
pub struct PhaseFlipCode {
    pub register: QuantumRegister,
}

impl PhaseFlipCode {
    /// Encode α|0⟩+β|1⟩ into 3-qubit phase-flip code.
    pub fn encode(alpha: (f64, f64), beta: (f64, f64)) -> Self {
        let mut reg = QuantumRegister::new(3, "phase_flip");
        reg.set_amplitude(0b000, alpha);
        reg.set_amplitude(0b100, beta);
        reg.normalize();

        // First CNOT to create redundancy in computational basis
        reg.cnot(0, 1);
        reg.cnot(0, 2);

        // Rotate to Hadamard basis: |0⟩→|+⟩, |1⟩→|-⟩
        reg.hadamard(0);
        reg.hadamard(1);
        reg.hadamard(2);

        Self { register: reg }
    }

    /// Inject a phase-flip error on qubit `k`.
    pub fn inject_error(&mut self, k: usize) {
        assert!(k < 3);
        self.register.pauli_z(k);
    }

    /// Syndrome measurement + correction.
    pub fn correct(&mut self) -> PhaseFlipSyndrome {
        // Transform back to computational basis to measure syndrome
        self.register.hadamard(0);
        self.register.hadamard(1);
        self.register.hadamard(2);

        let view = self.register.peek();

        let mut syndrome_counts = [0u32; 4];
        for sv in &view.states {
            let s1 = sv.bits[0] ^ sv.bits[1];
            let s2 = sv.bits[0] ^ sv.bits[2];
            let idx = (s1 as usize) * 2 + s2 as usize;
            syndrome_counts[idx] += 1;
        }

        let max_idx = syndrome_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
            .unwrap_or(0);

        let s1 = (max_idx >> 1) as u8;
        let s2 = (max_idx & 1) as u8;

        let corrected_qubit = match (s1, s2) {
            (0, 0) => None,
            (1, 1) => { self.register.pauli_x(0); Some(0) }
            (1, 0) => { self.register.pauli_x(1); Some(1) }
            (0, 1) => { self.register.pauli_x(2); Some(2) }
            _ => None,
        };

        // Restore Hadamard basis
        self.register.hadamard(0);
        self.register.hadamard(1);
        self.register.hadamard(2);

        PhaseFlipSyndrome {
            s1,
            s2,
            corrected_qubit,
        }
    }

    /// Decode: probability of logical |0⟩ and |1⟩.
    pub fn decode(&self) -> (f64, f64) {
        // Temporarily transform to computational basis to read
        let mut temp = self.register.fork();
        temp.hadamard(0);
        temp.hadamard(1);
        temp.hadamard(2);

        let view = temp.peek();
        let mut p_zero = 0.0;
        let mut p_one = 0.0;
        for sv in &view.states {
            if sv.bits == [0, 0, 0] {
                p_zero += sv.probability;
            } else if sv.bits == [1, 1, 1] {
                p_one += sv.probability;
            }
        }
        (p_zero, p_one)
    }
}

/// Syndrome for phase-flip code.
pub struct PhaseFlipSyndrome {
    pub s1: u8,
    pub s2: u8,
    pub corrected_qubit: Option<usize>,
}

// ═════════════════════════════════════════════════════════════════════
// Shor's 9-Qubit Code
// ═════════════════════════════════════════════════════════════════════

/// Shor's 9-qubit code — corrects arbitrary single-qubit errors.
///
/// Combines bit-flip and phase-flip protection:
/// - Outer code: phase-flip (3 blocks of 3 qubits)
/// - Inner code: bit-flip (within each block)
///
/// |0⟩_L = (|000⟩+|111⟩)(|000⟩+|111⟩)(|000⟩+|111⟩) / 2√2
/// |1⟩_L = (|000⟩-|111⟩)(|000⟩-|111⟩)(|000⟩-|111⟩) / 2√2
pub struct ShorCode {
    pub register: QuantumRegister,
}

impl ShorCode {
    /// Encode α|0⟩+β|1⟩ into Shor's 9-qubit code.
    pub fn encode(alpha: (f64, f64), beta: (f64, f64)) -> Self {
        let mut reg = QuantumRegister::new(9, "shor_code");
        reg.set_amplitude(0, alpha);
        reg.set_amplitude(1 << 8, beta); // |100000000⟩
        reg.normalize();

        // Phase-flip encoding: q0 → q0, q3, q6
        reg.cnot(0, 3);
        reg.cnot(0, 6);

        // Hadamard on block leaders
        reg.hadamard(0);
        reg.hadamard(3);
        reg.hadamard(6);

        // Bit-flip encoding within each block
        reg.cnot(0, 1);
        reg.cnot(0, 2);
        reg.cnot(3, 4);
        reg.cnot(3, 5);
        reg.cnot(6, 7);
        reg.cnot(6, 8);

        Self { register: reg }
    }

    /// Inject a bit-flip (X) error on qubit `k`.
    pub fn inject_x_error(&mut self, k: usize) {
        assert!(k < 9);
        self.register.pauli_x(k);
    }

    /// Inject a phase-flip (Z) error on qubit `k`.
    pub fn inject_z_error(&mut self, k: usize) {
        assert!(k < 9);
        self.register.pauli_z(k);
    }

    /// Inject both X and Z errors (Y error) on qubit `k`.
    pub fn inject_y_error(&mut self, k: usize) {
        assert!(k < 9);
        self.register.pauli_x(k);
        self.register.pauli_z(k);
    }

    /// Correct errors using syndrome measurement.
    ///
    /// Corrects any single-qubit error (X, Z, or Y = XZ).
    pub fn correct(&mut self) -> ShorSyndrome {
        let mut bit_corrections = Vec::new();
        let mut phase_corrections = Vec::new();

        // Step 1: Bit-flip correction within each block
        for block in 0..3 {
            let base = block * 3;
            let view = self.register.peek();

            let mut syndrome_counts = [0u32; 4];
            for sv in &view.states {
                let q0 = sv.bits[base];
                let q1 = sv.bits[base + 1];
                let q2 = sv.bits[base + 2];
                let s1 = q0 ^ q1;
                let s2 = q0 ^ q2;
                let idx = (s1 as usize) * 2 + s2 as usize;
                syndrome_counts[idx] += 1;
            }

            let max_idx = syndrome_counts
                .iter()
                .enumerate()
                .max_by_key(|(_, &c)| c)
                .map(|(i, _)| i)
                .unwrap_or(0);

            let s1 = (max_idx >> 1) as u8;
            let s2 = (max_idx & 1) as u8;

            match (s1, s2) {
                (1, 1) => {
                    self.register.pauli_x(base);
                    bit_corrections.push(base);
                }
                (1, 0) => {
                    self.register.pauli_x(base + 1);
                    bit_corrections.push(base + 1);
                }
                (0, 1) => {
                    self.register.pauli_x(base + 2);
                    bit_corrections.push(base + 2);
                }
                _ => {}
            }
        }

        // Step 2: Phase-flip correction between blocks
        // Decode bit-flip temporarily
        reg_decode_bitflip(&mut self.register);

        // Phase-flip syndrome: compare block leaders in Hadamard basis
        // Transform to computational basis
        self.register.hadamard(0);
        self.register.hadamard(3);
        self.register.hadamard(6);

        let view2 = self.register.peek();
        let mut phase_syndrome = [0u32; 4];
        for sv in &view2.states {
            let b0 = sv.bits[0];
            let b1 = sv.bits[3];
            let b2 = sv.bits[6];
            let ps1 = b0 ^ b1;
            let ps2 = b0 ^ b2;
            let idx = (ps1 as usize) * 2 + ps2 as usize;
            phase_syndrome[idx] += 1;
        }

        let max_idx = phase_syndrome
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
            .unwrap_or(0);

        let ps1 = (max_idx >> 1) as u8;
        let ps2 = (max_idx & 1) as u8;

        match (ps1, ps2) {
            (1, 1) => {
                self.register.pauli_x(0);
                phase_corrections.push(0);
            }
            (1, 0) => {
                self.register.pauli_x(3);
                phase_corrections.push(3);
            }
            (0, 1) => {
                self.register.pauli_x(6);
                phase_corrections.push(6);
            }
            _ => {}
        }

        // Restore Hadamard basis
        self.register.hadamard(0);
        self.register.hadamard(3);
        self.register.hadamard(6);

        // Re-encode bit-flip
        reg_encode_bitflip(&mut self.register);

        ShorSyndrome {
            bit_corrections,
            phase_corrections,
        }
    }

    /// Decode: probability of logical |0⟩ and |1⟩.
    pub fn decode(&self) -> (f64, f64) {
        let mut temp = self.register.fork();

        // Undo bit-flip encoding
        temp.cnot(6, 8);
        temp.cnot(6, 7);
        temp.cnot(3, 5);
        temp.cnot(3, 4);
        temp.cnot(0, 2);
        temp.cnot(0, 1);

        // Undo Hadamard
        temp.hadamard(0);
        temp.hadamard(3);
        temp.hadamard(6);

        // Undo phase-flip encoding
        temp.cnot(0, 6);
        temp.cnot(0, 3);

        let view = temp.peek();
        let mut p_zero = 0.0;
        let mut p_one = 0.0;
        for sv in &view.states {
            // Logical |0⟩ = qubit 0 is 0 (all ancillas should be 0)
            if sv.bits[0] == 0 {
                p_zero += sv.probability;
            } else {
                p_one += sv.probability;
            }
        }
        (p_zero, p_one)
    }
}

/// Syndrome result for Shor's code.
pub struct ShorSyndrome {
    pub bit_corrections: Vec<usize>,
    pub phase_corrections: Vec<usize>,
}

/// Helper: decode bit-flip within each block (CNOT reversal).
fn reg_decode_bitflip(reg: &mut QuantumRegister) {
    reg.cnot(6, 8);
    reg.cnot(6, 7);
    reg.cnot(3, 5);
    reg.cnot(3, 4);
    reg.cnot(0, 2);
    reg.cnot(0, 1);
}

/// Helper: re-encode bit-flip within each block.
fn reg_encode_bitflip(reg: &mut QuantumRegister) {
    reg.cnot(0, 1);
    reg.cnot(0, 2);
    reg.cnot(3, 4);
    reg.cnot(3, 5);
    reg.cnot(6, 7);
    reg.cnot(6, 8);
}

// ═════════════════════════════════════════════════════════════════════
// Steane's 7-Qubit Code
// ═════════════════════════════════════════════════════════════════════

/// Steane's [[7,1,3]] code — a CSS code that corrects arbitrary single-qubit errors.
///
/// Based on the classical [7,4,3] Hamming code.
/// |0⟩_L and |1⟩_L are superpositions of codewords.
pub struct SteaneCode {
    pub register: QuantumRegister,
}

impl SteaneCode {
    /// Encode α|0⟩+β|1⟩ into the 7-qubit Steane code.
    pub fn encode(alpha: (f64, f64), beta: (f64, f64)) -> Self {
        let mut reg = QuantumRegister::new(7, "steane_code");

        // Build |0⟩_L and |1⟩_L directly
        // |0⟩_L = (1/√8)(|0000000⟩+|1010101⟩+|0110011⟩+|1100110⟩
        //          +|0001111⟩+|1011010⟩+|0111100⟩+|1101001⟩)
        // |1⟩_L = X_all|0⟩_L (bitwise complement)

        let zero_codewords: [usize; 8] = [
            0b0000000, 0b1010101, 0b0110011, 0b1100110,
            0b0001111, 0b1011010, 0b0111100, 0b1101001,
        ];

        let inv_sqrt8 = 1.0 / (8.0f64).sqrt();

        // Clear all amplitudes
        for i in 0..128 {
            reg.set_amplitude(i, (0.0, 0.0));
        }

        for &cw in &zero_codewords {
            let complement = cw ^ 0b1111111;
            let amp_zero = (inv_sqrt8 * alpha.0, inv_sqrt8 * alpha.1);
            let amp_one = (inv_sqrt8 * beta.0, inv_sqrt8 * beta.1);

            let old0 = reg.amplitude(cw);
            reg.set_amplitude(cw, (old0.0 + amp_zero.0, old0.1 + amp_zero.1));

            let old1 = reg.amplitude(complement);
            reg.set_amplitude(complement, (old1.0 + amp_one.0, old1.1 + amp_one.1));
        }

        reg.normalize();

        Self { register: reg }
    }

    /// Inject a bit-flip error on qubit `k`.
    pub fn inject_x_error(&mut self, k: usize) {
        assert!(k < 7);
        self.register.pauli_x(k);
    }

    /// Inject a phase-flip error on qubit `k`.
    pub fn inject_z_error(&mut self, k: usize) {
        assert!(k < 7);
        self.register.pauli_z(k);
    }

    /// Correct errors.
    ///
    /// Uses the Hamming parity-check matrix to identify the error location.
    pub fn correct(&mut self) -> SteaneSyndrome {
        // X-error syndrome (from Z-stabilizers)
        // Parity check rows for [7,4,3] Hamming:
        // H = [[1,0,1,0,1,0,1],
        //      [0,1,1,0,0,1,1],
        //      [0,0,0,1,1,1,1]]
        // Standard Hamming [7,4,3] check matrix:
        // Row 0: positions 0,2,4,6 (1-indexed: 1,3,5,7)
        // Row 1: positions 1,2,5,6 (1-indexed: 2,3,6,7)
        // Row 2: positions 3,4,5,6 (1-indexed: 4,5,6,7)
        let check_positions: [Vec<usize>; 3] = [
            vec![0, 2, 4, 6],
            vec![1, 2, 5, 6],
            vec![3, 4, 5, 6],
        ];

        let view = self.register.peek();

        // Compute X-error syndrome
        let mut x_syndrome = 0usize;
        for (row_idx, positions) in check_positions.iter().enumerate() {
            let mut parity_counts = [0u32; 2]; // even, odd
            for sv in &view.states {
                let parity: u8 = positions.iter().map(|&p| sv.bits[p]).sum::<u8>() % 2;
                parity_counts[parity as usize] += 1;
            }
            if parity_counts[1] > parity_counts[0] {
                x_syndrome |= 1 << row_idx;
            }
        }

        let x_corrected = if x_syndrome > 0 && x_syndrome <= 7 {
            // Syndrome directly gives the error position (1-indexed in Hamming)
            let error_pos = x_syndrome - 1;
            self.register.pauli_x(error_pos);
            Some(error_pos)
        } else {
            None
        };

        // Z-error syndrome: transform to X basis, check, transform back
        for k in 0..7 {
            self.register.hadamard(k);
        }

        let view_h = self.register.peek();
        let mut z_syndrome = 0usize;
        for (row_idx, positions) in check_positions.iter().enumerate() {
            let mut parity_counts = [0u32; 2];
            for sv in &view_h.states {
                let parity: u8 = positions.iter().map(|&p| sv.bits[p]).sum::<u8>() % 2;
                parity_counts[parity as usize] += 1;
            }
            if parity_counts[1] > parity_counts[0] {
                z_syndrome |= 1 << row_idx;
            }
        }

        let z_corrected = if z_syndrome > 0 && z_syndrome <= 7 {
            let error_pos = z_syndrome - 1;
            self.register.pauli_x(error_pos); // X in H basis = Z in original
            Some(error_pos)
        } else {
            None
        };

        for k in 0..7 {
            self.register.hadamard(k);
        }

        SteaneSyndrome {
            x_syndrome,
            z_syndrome,
            x_corrected,
            z_corrected,
        }
    }

    /// Decode: probability of logical |0⟩ and |1⟩.
    pub fn decode(&self) -> (f64, f64) {
        let zero_codewords: [usize; 8] = [
            0b0000000, 0b1010101, 0b0110011, 0b1100110,
            0b0001111, 0b1011010, 0b0111100, 0b1101001,
        ];

        let view = self.register.peek();
        let mut p_zero = 0.0;
        let mut p_one = 0.0;

        for sv in &view.states {
            if zero_codewords.iter().any(|&cw| cw as u64 == sv.index) {
                p_zero += sv.probability;
            } else {
                p_one += sv.probability;
            }
        }
        (p_zero, p_one)
    }
}

/// Syndrome result for Steane's code.
pub struct SteaneSyndrome {
    pub x_syndrome: usize,
    pub z_syndrome: usize,
    pub x_corrected: Option<usize>,
    pub z_corrected: Option<usize>,
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

    // ── Bit-Flip Code ───────────────────────────────────────────

    #[test]
    fn test_bitflip_no_error() {
        let code = BitFlipCode::encode((1.0, 0.0), (0.0, 0.0));
        let (p0, p1) = code.decode();
        assert!(approx(p0, 1.0, 1e-10));
        assert!(approx(p1, 0.0, 1e-10));
    }

    #[test]
    fn test_bitflip_correct_q0() {
        let mut code = BitFlipCode::encode((1.0, 0.0), (0.0, 0.0));
        code.inject_error(0);
        let syndrome = code.correct();
        assert_eq!(syndrome.corrected_qubit, Some(0));
        let (p0, _) = code.decode();
        assert!(approx(p0, 1.0, 1e-10));
    }

    #[test]
    fn test_bitflip_correct_q1() {
        let mut code = BitFlipCode::encode((0.0, 0.0), (1.0, 0.0));
        code.inject_error(1);
        let syndrome = code.correct();
        assert_eq!(syndrome.corrected_qubit, Some(1));
        let (_, p1) = code.decode();
        assert!(approx(p1, 1.0, 1e-10));
    }

    #[test]
    fn test_bitflip_correct_q2() {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let mut code = BitFlipCode::encode((s, 0.0), (s, 0.0));
        code.inject_error(2);
        let syndrome = code.correct();
        assert_eq!(syndrome.corrected_qubit, Some(2));
        let (p0, p1) = code.decode();
        assert!(approx(p0, 0.5, 1e-10));
        assert!(approx(p1, 0.5, 1e-10));
    }

    // ── Phase-Flip Code ─────────────────────────────────────────

    #[test]
    fn test_phaseflip_no_error() {
        let code = PhaseFlipCode::encode((1.0, 0.0), (0.0, 0.0));
        let (p0, p1) = code.decode();
        assert!(approx(p0, 1.0, 1e-8));
        assert!(approx(p1, 0.0, 1e-8));
    }

    #[test]
    fn test_phaseflip_correct_q0() {
        let mut code = PhaseFlipCode::encode((1.0, 0.0), (0.0, 0.0));
        code.inject_error(0);
        let syndrome = code.correct();
        assert_eq!(syndrome.corrected_qubit, Some(0));
        let (p0, _) = code.decode();
        assert!(approx(p0, 1.0, 1e-8));
    }

    #[test]
    fn test_phaseflip_correct_q1() {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let mut code = PhaseFlipCode::encode((s, 0.0), (s, 0.0));
        code.inject_error(1);
        let _syndrome = code.correct();
        let (p0, p1) = code.decode();
        assert!(approx(p0 + p1, 1.0, 1e-6));
    }

    // ── Shor's 9-Qubit Code ────────────────────────────────────

    #[test]
    fn test_shor_no_error() {
        let code = ShorCode::encode((1.0, 0.0), (0.0, 0.0));
        let (p0, p1) = code.decode();
        assert!(approx(p0, 1.0, 1e-8));
        assert!(approx(p1, 0.0, 1e-8));
    }

    #[test]
    fn test_shor_correct_x_error() {
        let mut code = ShorCode::encode((1.0, 0.0), (0.0, 0.0));
        code.inject_x_error(4);
        let syndrome = code.correct();
        assert!(!syndrome.bit_corrections.is_empty());
        let (p0, _) = code.decode();
        assert!(approx(p0, 1.0, 1e-6));
    }

    #[test]
    fn test_shor_correct_z_error() {
        let mut code = ShorCode::encode((1.0, 0.0), (0.0, 0.0));
        code.inject_z_error(0);
        let syndrome = code.correct();
        assert!(!syndrome.phase_corrections.is_empty() || !syndrome.bit_corrections.is_empty());
        let (p0, _) = code.decode();
        assert!(approx(p0, 1.0, 1e-6));
    }

    #[test]
    fn test_shor_encode_superposition() {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let code = ShorCode::encode((s, 0.0), (s, 0.0));
        let (p0, p1) = code.decode();
        assert!(approx(p0, 0.5, 1e-6));
        assert!(approx(p1, 0.5, 1e-6));
    }

    // ── Steane's 7-Qubit Code ──────────────────────────────────

    #[test]
    fn test_steane_no_error() {
        let code = SteaneCode::encode((1.0, 0.0), (0.0, 0.0));
        let (p0, p1) = code.decode();
        assert!(approx(p0, 1.0, 1e-8));
        assert!(approx(p1, 0.0, 1e-8));
    }

    #[test]
    fn test_steane_encode_one() {
        let code = SteaneCode::encode((0.0, 0.0), (1.0, 0.0));
        let (p0, p1) = code.decode();
        assert!(approx(p0, 0.0, 1e-8));
        assert!(approx(p1, 1.0, 1e-8));
    }

    #[test]
    fn test_steane_correct_x_error() {
        let mut code = SteaneCode::encode((1.0, 0.0), (0.0, 0.0));
        code.inject_x_error(3);
        let syndrome = code.correct();
        assert!(syndrome.x_corrected.is_some());
        let (p0, _) = code.decode();
        assert!(approx(p0, 1.0, 1e-6));
    }

    #[test]
    fn test_steane_correct_z_error() {
        let mut code = SteaneCode::encode((1.0, 0.0), (0.0, 0.0));
        code.inject_z_error(2);
        let syndrome = code.correct();
        assert!(syndrome.z_corrected.is_some());
        let (p0, _) = code.decode();
        assert!(approx(p0, 1.0, 1e-6));
    }

    #[test]
    fn test_steane_superposition() {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let code = SteaneCode::encode((s, 0.0), (s, 0.0));
        let (p0, p1) = code.decode();
        assert!(approx(p0, 0.5, 1e-6));
        assert!(approx(p1, 0.5, 1e-6));
    }
}
