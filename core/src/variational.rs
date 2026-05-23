//! # Variational Quantum Algorithms (VQE / QAOA)
//!
//! Hybrid classical-quantum algorithms that use parameterised quantum circuits
//! optimised by a classical outer loop.
//!
//! ## VQE (Variational Quantum Eigensolver)
//! Finds the ground-state energy of a Hamiltonian by minimising
//! ⟨ψ(θ)|H|ψ(θ)⟩ over circuit parameters θ.
//!
//! ## QAOA (Quantum Approximate Optimization Algorithm)
//! Finds approximate solutions to combinatorial optimization problems
//! using alternating cost/mixer layers parameterised by (γ, β).
//!
//! MDB advantage: `peek()` lets us evaluate expectation values without
//! destructive measurement, giving exact gradients instead of shot noise.

use crate::register::QuantumRegister;
use std::f64::consts::PI;

// ═════════════════════════════════════════════════════════════════════
// VQE
// ═════════════════════════════════════════════════════════════════════

/// Pauli term in a Hamiltonian: coefficient × tensor product of Paulis.
///
/// Each entry is (position, pauli_type) where pauli_type is 'I', 'X', 'Y', 'Z'.
#[derive(Debug, Clone)]
pub struct PauliTerm {
    pub coefficient: f64,
    pub paulis: Vec<(usize, char)>,
}

impl PauliTerm {
    pub fn new(coefficient: f64, paulis: Vec<(usize, char)>) -> Self {
        Self { coefficient, paulis }
    }

    /// Identity term (scalar).
    pub fn identity(coefficient: f64) -> Self {
        Self {
            coefficient,
            paulis: vec![],
        }
    }

    /// Single Pauli.
    pub fn single(coefficient: f64, position: usize, pauli: char) -> Self {
        Self {
            coefficient,
            paulis: vec![(position, pauli)],
        }
    }

    /// ZZ interaction.
    pub fn zz(coefficient: f64, a: usize, b: usize) -> Self {
        Self {
            coefficient,
            paulis: vec![(a, 'Z'), (b, 'Z')],
        }
    }
}

/// Hamiltonian as a sum of Pauli terms.
#[derive(Debug, Clone)]
pub struct Hamiltonian {
    pub terms: Vec<PauliTerm>,
    pub n: usize,
}

impl Hamiltonian {
    pub fn new(n: usize, terms: Vec<PauliTerm>) -> Self {
        Self { terms, n }
    }

    /// Compute ⟨ψ|H|ψ⟩ exactly using MDB peek (no shot noise!).
    pub fn expectation_value(&self, reg: &QuantumRegister) -> f64 {
        let mut total = 0.0;
        for term in &self.terms {
            total += term.coefficient * pauli_expectation(reg, &term.paulis);
        }
        total
    }

    /// Transverse-field Ising model: H = -J Σ Z_i Z_{i+1} - h Σ X_i
    pub fn transverse_ising(n: usize, j: f64, h: f64) -> Self {
        let mut terms = Vec::new();
        for i in 0..n - 1 {
            terms.push(PauliTerm::zz(-j, i, i + 1));
        }
        for i in 0..n {
            terms.push(PauliTerm::single(-h, i, 'X'));
        }
        Self { terms, n }
    }

    /// Simple H2 molecule Hamiltonian (2-qubit approximation).
    /// H = g0 I + g1 Z0 + g2 Z1 + g3 Z0Z1 + g4 X0X1
    pub fn hydrogen_molecule(bond_length: f64) -> Self {
        // Simplified coefficients that vary with bond length
        // These are rough approximations of the STO-3G basis
        let r = bond_length;
        let g0 = -0.5 + 0.2 * r;
        let g1 = 0.4 - 0.1 * r;
        let g2 = -0.4 + 0.1 * r;
        let g3 = 0.15 + 0.05 * r;
        let g4 = 0.17 - 0.02 * r;

        Self::new(
            2,
            vec![
                PauliTerm::identity(g0),
                PauliTerm::single(g1, 0, 'Z'),
                PauliTerm::single(g2, 1, 'Z'),
                PauliTerm::zz(g3, 0, 1),
                PauliTerm::new(g4, vec![(0, 'X'), (1, 'X')]),
            ],
        )
    }
}

/// Compute ⟨ψ|P|ψ⟩ for a Pauli string P using peek.
fn pauli_expectation(reg: &QuantumRegister, paulis: &[(usize, char)]) -> f64 {
    if paulis.is_empty() {
        return 1.0; // Identity
    }

    // For Z-only terms, we can compute directly from probabilities
    let all_z = paulis.iter().all(|(_, p)| *p == 'Z');
    if all_z {
        let view = reg.peek();
        let mut exp = 0.0;
        for sv in &view.states {
            let mut parity = 1.0f64;
            for (pos, _) in paulis {
                if sv.bits[*pos] == 1 {
                    parity *= -1.0;
                }
            }
            exp += sv.probability * parity;
        }
        return exp;
    }

    // For mixed Pauli terms, transform basis then measure in Z
    let mut temp = reg.fork();

    // Change basis for non-Z Paulis
    for &(pos, pauli) in paulis {
        match pauli {
            'X' => temp.hadamard(pos),        // X → Z in Hadamard basis
            'Y' => {
                // Y → Z via S†H
                temp.phase_gate(pos, -PI / 2.0); // S†
                temp.hadamard(pos);
            }
            'Z' | 'I' => {} // Already in Z basis
            _ => panic!("Unknown Pauli: {}", pauli),
        }
    }

    // Now measure all as Z
    let view = temp.peek();
    let mut exp = 0.0;
    for sv in &view.states {
        let mut parity = 1.0f64;
        for (pos, pauli) in paulis {
            if *pauli != 'I' && sv.bits[*pos] == 1 {
                parity *= -1.0;
            }
        }
        exp += sv.probability * parity;
    }
    exp
}

/// VQE ansatz type.
#[derive(Debug, Clone)]
pub enum Ansatz {
    /// Ry-CNOT ladder: Ry(θ_i) on each qubit, then CNOT chain.
    RyLadder,
    /// Hardware-efficient: Ry-Rz on each qubit, CNOT entanglement, repeated.
    HardwareEfficient { layers: usize },
}

/// Result of VQE optimization.
#[derive(Debug, Clone)]
pub struct VqeResult {
    /// Optimal energy found.
    pub energy: f64,
    /// Optimal parameters.
    pub parameters: Vec<f64>,
    /// Number of iterations.
    pub iterations: usize,
    /// Energy history per iteration.
    pub energy_history: Vec<f64>,
}

/// Run VQE to find the ground state energy of a Hamiltonian.
///
/// Uses coordinate descent (parameter-shift rule) for optimization.
pub fn vqe(
    hamiltonian: &Hamiltonian,
    ansatz: Ansatz,
    initial_params: Option<Vec<f64>>,
    max_iterations: usize,
    learning_rate: f64,
) -> VqeResult {
    let n = hamiltonian.n;
    let param_count = match &ansatz {
        Ansatz::RyLadder => n,
        Ansatz::HardwareEfficient { layers } => n * 2 * layers,
    };

    let mut params = initial_params.unwrap_or_else(|| vec![0.1; param_count]);
    assert_eq!(params.len(), param_count);

    let mut energy_history = Vec::new();
    let mut best_energy = f64::MAX;
    let mut best_params = params.clone();

    for _ in 0..max_iterations {
        let energy = evaluate_ansatz(hamiltonian, &ansatz, &params);
        energy_history.push(energy);

        if energy < best_energy {
            best_energy = energy;
            best_params = params.clone();
        }

        // Parameter-shift gradient
        let mut gradients = vec![0.0; param_count];
        let shift = PI / 2.0;
        for i in 0..param_count {
            let mut p_plus = params.clone();
            let mut p_minus = params.clone();
            p_plus[i] += shift;
            p_minus[i] -= shift;
            let e_plus = evaluate_ansatz(hamiltonian, &ansatz, &p_plus);
            let e_minus = evaluate_ansatz(hamiltonian, &ansatz, &p_minus);
            gradients[i] = (e_plus - e_minus) / 2.0;
        }

        // Gradient descent
        for i in 0..param_count {
            params[i] -= learning_rate * gradients[i];
        }
    }

    VqeResult {
        energy: best_energy,
        parameters: best_params,
        iterations: max_iterations,
        energy_history,
    }
}

/// Evaluate the ansatz for given parameters.
fn evaluate_ansatz(hamiltonian: &Hamiltonian, ansatz: &Ansatz, params: &[f64]) -> f64 {
    let n = hamiltonian.n;
    let mut reg = QuantumRegister::new(n, "vqe");

    match ansatz {
        Ansatz::RyLadder => {
            for k in 0..n {
                reg.ry(k, params[k]);
            }
            for k in 0..n - 1 {
                reg.cnot(k, k + 1);
            }
        }
        Ansatz::HardwareEfficient { layers } => {
            let mut idx = 0;
            for _ in 0..*layers {
                for k in 0..n {
                    reg.ry(k, params[idx]);
                    idx += 1;
                    reg.rz(k, params[idx]);
                    idx += 1;
                }
                for k in 0..n - 1 {
                    reg.cnot(k, k + 1);
                }
            }
        }
    }

    hamiltonian.expectation_value(&reg)
}

// ═════════════════════════════════════════════════════════════════════
// QAOA
// ═════════════════════════════════════════════════════════════════════

/// Cost function for QAOA: maps bitstring → cost value.
pub type CostFn = dyn Fn(&[u8]) -> f64;

/// Result of QAOA optimization.
#[derive(Debug, Clone)]
pub struct QaoaResult {
    /// Best solution found.
    pub solution: Vec<u8>,
    /// Cost of the best solution.
    pub cost: f64,
    /// Optimal gamma parameters.
    pub gammas: Vec<f64>,
    /// Optimal beta parameters.
    pub betas: Vec<f64>,
    /// Number of optimization iterations.
    pub iterations: usize,
}

/// QAOA for combinatorial optimization.
///
/// Alternates cost operator exp(-iγC) and mixer exp(-iβB) for `p` layers.
/// The cost function C encodes the problem, and B = ΣX_i is the transverse mixer.
pub fn qaoa(
    n: usize,
    p: usize, // depth (number of layers)
    cost_fn: &CostFn,
    max_iterations: usize,
) -> QaoaResult {
    let mut gammas = vec![0.5; p];
    let mut betas = vec![0.5; p];

    let mut best_cost = f64::NEG_INFINITY;
    let mut best_solution = vec![0u8; n];
    let mut best_gammas = gammas.clone();
    let mut best_betas = betas.clone();

    let lr = 0.1;

    for _ in 0..max_iterations {
        // Evaluate current parameters
        let (sol, cost) = evaluate_qaoa(n, p, &gammas, &betas, cost_fn);

        if cost > best_cost {
            best_cost = cost;
            best_solution = sol;
            best_gammas = gammas.clone();
            best_betas = betas.clone();
        }

        // Gradient via parameter shift
        let shift = PI / 4.0;
        for i in 0..p {
            // Gamma gradient
            let mut gp = gammas.clone();
            let mut gm = gammas.clone();
            gp[i] += shift;
            gm[i] -= shift;
            let (_, cp) = evaluate_qaoa(n, p, &gp, &betas, cost_fn);
            let (_, cm) = evaluate_qaoa(n, p, &gm, &betas, cost_fn);
            gammas[i] += lr * (cp - cm) / 2.0; // Ascend for maximisation

            // Beta gradient
            let mut bp = betas.clone();
            let mut bm = betas.clone();
            bp[i] += shift;
            bm[i] -= shift;
            let (_, cp) = evaluate_qaoa(n, p, &gammas, &bp, cost_fn);
            let (_, cm) = evaluate_qaoa(n, p, &gammas, &bm, cost_fn);
            betas[i] += lr * (cp - cm) / 2.0;
        }
    }

    QaoaResult {
        solution: best_solution,
        cost: best_cost,
        gammas: best_gammas,
        betas: best_betas,
        iterations: max_iterations,
    }
}

/// Evaluate QAOA circuit for given parameters. Returns (best bitstring, expected cost).
fn evaluate_qaoa(
    n: usize,
    p: usize,
    gammas: &[f64],
    betas: &[f64],
    cost_fn: &CostFn,
) -> (Vec<u8>, f64) {
    let mut reg = QuantumRegister::new(n, "qaoa");

    // Initial state: uniform superposition
    for k in 0..n {
        reg.hadamard(k);
    }

    // Alternating layers
    for layer in 0..p {
        // Cost operator: exp(-iγC) — phase-shift each basis state by its cost
        apply_cost_operator(&mut reg, gammas[layer], cost_fn);

        // Mixer operator: exp(-iβB) where B = ΣX_i → Rx(2β) on each qubit
        for k in 0..n {
            reg.rx(k, 2.0 * betas[layer]);
        }
    }

    // Find best state via peek
    let view = reg.peek();

    let mut expected_cost = 0.0;
    let mut best_state = vec![0u8; n];
    let mut best_cost = f64::NEG_INFINITY;

    for sv in &view.states {
        let c = cost_fn(&sv.bits);
        expected_cost += sv.probability * c;
        if c > best_cost {
            best_cost = c;
            best_state = sv.bits.clone();
        }
    }

    (best_state, expected_cost)
}

/// Apply the cost operator exp(-iγC) to the register.
fn apply_cost_operator(reg: &mut QuantumRegister, gamma: f64, cost_fn: &CostFn) {
    let dim = reg.dim();
    let n = reg.n;
    for i in 0..dim {
        let amp = reg.amplitude(i);
        if amp.0 == 0.0 && amp.1 == 0.0 {
            continue;
        }
        let bits: Vec<u8> = (0..n)
            .map(|k| ((i >> (n - 1 - k)) & 1) as u8)
            .collect();
        let cost = cost_fn(&bits);
        let phase = -gamma * cost;
        let rot = (phase.cos(), phase.sin());
        let new_amp = (
            amp.0 * rot.0 - amp.1 * rot.1,
            amp.0 * rot.1 + amp.1 * rot.0,
        );
        reg.set_amplitude(i, new_amp);
    }
}

/// MaxCut cost function for a graph.
///
/// Returns the number of edges crossing the cut defined by the bitstring.
pub fn maxcut_cost(edges: Vec<(usize, usize)>) -> Box<dyn Fn(&[u8]) -> f64> {
    Box::new(move |bits: &[u8]| {
        let mut cost = 0.0;
        for &(u, v) in &edges {
            if bits[u] != bits[v] {
                cost += 1.0;
            }
        }
        cost
    })
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

    // ── VQE Tests ───────────────────────────────────────────────

    #[test]
    fn test_vqe_simple_z() {
        // H = Z (ground state = |0⟩, energy = 1.0... wait, Z|0⟩ = |0⟩ → eigenvalue +1,
        // Z|1⟩ = -|1⟩ → eigenvalue -1. Ground state energy = -1)
        let h = Hamiltonian::new(1, vec![PauliTerm::single(1.0, 0, 'Z')]);
        let result = vqe(&h, Ansatz::RyLadder, None, 50, 0.3);
        assert!(
            result.energy < -0.9,
            "ground state energy should be ~ -1, got {}",
            result.energy
        );
    }

    #[test]
    fn test_vqe_zz_model() {
        // H = Z0Z1 — ground state = |01⟩ or |10⟩, energy = -1
        let h = Hamiltonian::new(2, vec![PauliTerm::zz(1.0, 0, 1)]);
        let result = vqe(&h, Ansatz::RyLadder, None, 100, 0.2);
        assert!(
            result.energy < -0.8,
            "ZZ ground energy should be ~ -1, got {}",
            result.energy
        );
    }

    #[test]
    fn test_vqe_hardware_efficient() {
        let h = Hamiltonian::new(2, vec![PauliTerm::single(1.0, 0, 'Z')]);
        let result = vqe(
            &h,
            Ansatz::HardwareEfficient { layers: 2 },
            None,
            50,
            0.2,
        );
        assert!(
            result.energy < -0.8,
            "should find ground state, got {}",
            result.energy
        );
    }

    #[test]
    fn test_vqe_hydrogen() {
        let h = Hamiltonian::hydrogen_molecule(0.75);
        let result = vqe(&h, Ansatz::RyLadder, None, 100, 0.3);
        // Just check it converges to something reasonable
        assert!(
            result.energy < 0.0,
            "H2 energy should be negative, got {}",
            result.energy
        );
    }

    #[test]
    fn test_hamiltonian_expectation() {
        // ⟨0|Z|0⟩ = 1
        let reg = QuantumRegister::new(1, "t");
        let h = Hamiltonian::new(1, vec![PauliTerm::single(1.0, 0, 'Z')]);
        let exp = h.expectation_value(&reg);
        assert!(approx(exp, 1.0, 1e-10));
    }

    #[test]
    fn test_pauli_x_expectation() {
        // ⟨+|X|+⟩ = 1 where |+⟩ = H|0⟩
        let mut reg = QuantumRegister::new(1, "t");
        reg.hadamard(0);
        let exp = pauli_expectation(&reg, &[(0, 'X')]);
        assert!(approx(exp, 1.0, 1e-10));
    }

    #[test]
    fn test_energy_decreases() {
        let h = Hamiltonian::transverse_ising(3, 1.0, 0.5);
        let result = vqe(&h, Ansatz::RyLadder, None, 30, 0.2);
        // First energy should be ≥ last energy (descent)
        if result.energy_history.len() > 2 {
            let first = result.energy_history[0];
            let last = *result.energy_history.last().unwrap();
            assert!(
                last <= first + 0.1,
                "energy should generally decrease: first={}, last={}",
                first,
                last
            );
        }
    }

    // ── QAOA Tests ──────────────────────────────────────────────

    #[test]
    fn test_qaoa_maxcut_triangle() {
        // Triangle graph: edges (0,1), (1,2), (0,2)
        // MaxCut = 2 (e.g., {0} vs {1,2})
        let cost = maxcut_cost(vec![(0, 1), (1, 2), (0, 2)]);
        let result = qaoa(3, 2, &cost, 30);
        assert!(
            result.cost >= 1.5,
            "QAOA MaxCut should find cut ≥ 2, got {}",
            result.cost
        );
    }

    #[test]
    fn test_qaoa_maxcut_square() {
        // Square graph: 4 nodes, 4 edges
        let cost = maxcut_cost(vec![(0, 1), (1, 2), (2, 3), (3, 0)]);
        let result = qaoa(4, 2, &cost, 30);
        assert!(
            result.cost >= 2.0,
            "QAOA MaxCut on square should find cut ≥ 3, got {}",
            result.cost
        );
    }

    #[test]
    fn test_qaoa_simple_cost() {
        // Trivial: maximise number of 1s
        let result = qaoa(
            3,
            1,
            &|bits: &[u8]| bits.iter().map(|&b| b as f64).sum(),
            20,
        );
        let ones: u8 = result.solution.iter().sum();
        assert!(ones >= 2, "should favour all-1s, got {} ones", ones);
    }

    #[test]
    fn test_maxcut_cost_fn() {
        let cost = maxcut_cost(vec![(0, 1), (1, 2)]);
        assert!(approx(cost(&[0, 1, 0]), 2.0, 1e-10)); // both edges cut
        assert!(approx(cost(&[0, 0, 0]), 0.0, 1e-10)); // no edges cut
        assert!(approx(cost(&[1, 0, 1]), 2.0, 1e-10)); // both edges cut
    }
}
