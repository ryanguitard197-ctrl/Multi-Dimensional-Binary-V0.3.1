//! # Circuit Model
//!
//! Declarative quantum circuit builder with ASCII visualization.
//!
//! Build circuits by chaining gate operations, then execute on a
//! [`QuantumRegister`]. Circuits can be visualized, composed, repeated,
//! and measured — the standard way quantum algorithms are expressed.
//!
//! ```text
//!     q0: ─H──●──────────M─
//!             │
//!     q1: ────X──●───────M─
//!                │
//!     q2: ───────X──H────M─
//! ```

use crate::register::QuantumRegister;
use std::fmt;

// ── Gate Instruction ────────────────────────────────────────────────

/// A single gate operation in a circuit.
#[derive(Debug, Clone)]
pub enum GateOp {
    // Single-position gates
    Hadamard(usize),
    PauliX(usize),
    PauliY(usize),
    PauliZ(usize),
    SGate(usize),
    TGate(usize),
    Phase(usize, f64),
    Rx(usize, f64),
    Ry(usize, f64),
    Rz(usize, f64),

    // Two-position gates
    Cnot(usize, usize),
    Cz(usize, usize),
    Swap(usize, usize),
    CPhase(usize, usize, f64),

    // Three-position gates
    Toffoli(usize, usize, usize),
    Fredkin(usize, usize, usize),

    // Multi-position
    Qft(Vec<usize>),
    InverseQft(Vec<usize>),

    // Measurement
    Measure(usize),
    MeasureAll,

    // Barrier (visual separator, no operation)
    Barrier,

    // Custom label (for display)
    Custom(String, Vec<usize>),
}

impl GateOp {
    /// Positions this gate touches.
    fn positions(&self) -> Vec<usize> {
        match self {
            GateOp::Hadamard(k)
            | GateOp::PauliX(k)
            | GateOp::PauliY(k)
            | GateOp::PauliZ(k)
            | GateOp::SGate(k)
            | GateOp::TGate(k)
            | GateOp::Phase(k, _)
            | GateOp::Rx(k, _)
            | GateOp::Ry(k, _)
            | GateOp::Rz(k, _)
            | GateOp::Measure(k) => vec![*k],

            GateOp::Cnot(a, b) | GateOp::Cz(a, b) | GateOp::Swap(a, b) | GateOp::CPhase(a, b, _) => {
                vec![*a, *b]
            }

            GateOp::Toffoli(a, b, c) | GateOp::Fredkin(a, b, c) => vec![*a, *b, *c],

            GateOp::Qft(ps) | GateOp::InverseQft(ps) => ps.clone(),

            GateOp::MeasureAll | GateOp::Barrier => vec![],

            GateOp::Custom(_, ps) => ps.clone(),
        }
    }

    /// Short label for ASCII rendering.
    fn label(&self) -> &str {
        match self {
            GateOp::Hadamard(_) => "H",
            GateOp::PauliX(_) => "X",
            GateOp::PauliY(_) => "Y",
            GateOp::PauliZ(_) => "Z",
            GateOp::SGate(_) => "S",
            GateOp::TGate(_) => "T",
            GateOp::Phase(_, _) => "P",
            GateOp::Rx(_, _) => "Rx",
            GateOp::Ry(_, _) => "Ry",
            GateOp::Rz(_, _) => "Rz",
            GateOp::Cnot(_, _) => "CX",
            GateOp::Cz(_, _) => "CZ",
            GateOp::Swap(_, _) => "SW",
            GateOp::CPhase(_, _, _) => "CP",
            GateOp::Toffoli(_, _, _) => "CCX",
            GateOp::Fredkin(_, _, _) => "CSW",
            GateOp::Qft(_) => "QFT",
            GateOp::InverseQft(_) => "QFT†",
            GateOp::Measure(_) => "M",
            GateOp::MeasureAll => "M*",
            GateOp::Barrier => "|",
            GateOp::Custom(name, _) => name.as_str(),
        }
    }
}

// ── Circuit Result ──────────────────────────────────────────────────

/// Result of circuit execution.
pub struct CircuitResult {
    /// Classical measurement outcomes (position → value), in order measured.
    pub measurements: Vec<(usize, u8)>,
    /// The register after execution (still accessible for peek/fork).
    pub register: QuantumRegister,
    /// Number of gates executed (excluding barriers).
    pub gate_count: usize,
    /// Circuit depth (number of time steps).
    pub depth: usize,
}

// ── Circuit Builder ─────────────────────────────────────────────────

/// Declarative quantum circuit.
///
/// Build by chaining methods, then `execute()` on a register.
pub struct Circuit {
    /// Number of positions.
    pub n: usize,
    /// Human-readable name.
    pub name: String,
    /// Ordered list of operations.
    ops: Vec<GateOp>,
    /// Measurement seed.
    seed: u64,
}

impl Circuit {
    /// Create a new circuit for `n` positions.
    pub fn new(n: usize, name: &str) -> Self {
        assert!(n > 0 && n <= 24);
        Self {
            n,
            name: name.into(),
            ops: Vec::new(),
            seed: 42,
        }
    }

    /// Set the measurement seed for deterministic replay.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    // ── Single-position gates ───────────────────────────────────

    pub fn h(mut self, k: usize) -> Self {
        self.ops.push(GateOp::Hadamard(k));
        self
    }

    pub fn x(mut self, k: usize) -> Self {
        self.ops.push(GateOp::PauliX(k));
        self
    }

    pub fn y(mut self, k: usize) -> Self {
        self.ops.push(GateOp::PauliY(k));
        self
    }

    pub fn z(mut self, k: usize) -> Self {
        self.ops.push(GateOp::PauliZ(k));
        self
    }

    pub fn s(mut self, k: usize) -> Self {
        self.ops.push(GateOp::SGate(k));
        self
    }

    pub fn t(mut self, k: usize) -> Self {
        self.ops.push(GateOp::TGate(k));
        self
    }

    pub fn phase(mut self, k: usize, theta: f64) -> Self {
        self.ops.push(GateOp::Phase(k, theta));
        self
    }

    pub fn rx(mut self, k: usize, theta: f64) -> Self {
        self.ops.push(GateOp::Rx(k, theta));
        self
    }

    pub fn ry(mut self, k: usize, theta: f64) -> Self {
        self.ops.push(GateOp::Ry(k, theta));
        self
    }

    pub fn rz(mut self, k: usize, theta: f64) -> Self {
        self.ops.push(GateOp::Rz(k, theta));
        self
    }

    // ── Two-position gates ──────────────────────────────────────

    pub fn cnot(mut self, control: usize, target: usize) -> Self {
        self.ops.push(GateOp::Cnot(control, target));
        self
    }

    pub fn cx(self, control: usize, target: usize) -> Self {
        self.cnot(control, target)
    }

    pub fn cz(mut self, a: usize, b: usize) -> Self {
        self.ops.push(GateOp::Cz(a, b));
        self
    }

    pub fn swap(mut self, a: usize, b: usize) -> Self {
        self.ops.push(GateOp::Swap(a, b));
        self
    }

    pub fn cphase(mut self, control: usize, target: usize, theta: f64) -> Self {
        self.ops.push(GateOp::CPhase(control, target, theta));
        self
    }

    // ── Three-position gates ────────────────────────────────────

    pub fn toffoli(mut self, c1: usize, c2: usize, target: usize) -> Self {
        self.ops.push(GateOp::Toffoli(c1, c2, target));
        self
    }

    pub fn ccx(self, c1: usize, c2: usize, target: usize) -> Self {
        self.toffoli(c1, c2, target)
    }

    pub fn fredkin(mut self, control: usize, t1: usize, t2: usize) -> Self {
        self.ops.push(GateOp::Fredkin(control, t1, t2));
        self
    }

    // ── Multi-position ──────────────────────────────────────────

    pub fn qft(mut self, positions: &[usize]) -> Self {
        self.ops.push(GateOp::Qft(positions.to_vec()));
        self
    }

    pub fn inverse_qft(mut self, positions: &[usize]) -> Self {
        self.ops.push(GateOp::InverseQft(positions.to_vec()));
        self
    }

    // ── Measurement ─────────────────────────────────────────────

    pub fn measure(mut self, k: usize) -> Self {
        self.ops.push(GateOp::Measure(k));
        self
    }

    pub fn measure_all(mut self) -> Self {
        self.ops.push(GateOp::MeasureAll);
        self
    }

    // ── Barrier ─────────────────────────────────────────────────

    pub fn barrier(mut self) -> Self {
        self.ops.push(GateOp::Barrier);
        self
    }

    // ── Composition ─────────────────────────────────────────────

    /// Apply Hadamard to all positions.
    pub fn h_all(mut self) -> Self {
        for k in 0..self.n {
            self.ops.push(GateOp::Hadamard(k));
        }
        self
    }

    /// Repeat a sub-circuit `count` times.
    pub fn repeat(mut self, count: usize, builder: impl Fn(Circuit) -> Circuit) -> Self {
        for _ in 0..count {
            let sub = builder(Circuit::new(self.n, "sub"));
            self.ops.extend(sub.ops);
        }
        self
    }

    /// Append another circuit's operations.
    pub fn append(mut self, other: Circuit) -> Self {
        assert_eq!(self.n, other.n);
        self.ops.extend(other.ops);
        self
    }

    /// Number of non-barrier operations.
    pub fn gate_count(&self) -> usize {
        self.ops
            .iter()
            .filter(|op| !matches!(op, GateOp::Barrier))
            .count()
    }

    /// Circuit depth (each gate = 1 time step, parallel gates on different
    /// qubits could share a step but we count sequentially for simplicity).
    pub fn depth(&self) -> usize {
        self.ops
            .iter()
            .filter(|op| !matches!(op, GateOp::Barrier))
            .count()
    }

    /// Get the operations list (for inspection).
    pub fn operations(&self) -> &[GateOp] {
        &self.ops
    }

    // ── Execution ───────────────────────────────────────────────

    /// Execute the circuit on a fresh |0…0⟩ register.
    pub fn execute(self) -> CircuitResult {
        let reg = QuantumRegister::new(self.n, &self.name);
        self.execute_on(reg)
    }

    /// Execute on an existing register.
    pub fn execute_on(self, mut reg: QuantumRegister) -> CircuitResult {
        assert_eq!(reg.n, self.n);
        let mut measurements = Vec::new();
        let mut seed = self.seed;
        let gate_count = self.gate_count();
        let depth = self.depth();

        for op in &self.ops {
            match op {
                GateOp::Hadamard(k) => reg.hadamard(*k),
                GateOp::PauliX(k) => reg.pauli_x(*k),
                GateOp::PauliY(k) => reg.pauli_y(*k),
                GateOp::PauliZ(k) => reg.pauli_z(*k),
                GateOp::SGate(k) => reg.s_gate(*k),
                GateOp::TGate(k) => reg.t_gate(*k),
                GateOp::Phase(k, theta) => reg.phase_gate(*k, *theta),
                GateOp::Rx(k, theta) => reg.rx(*k, *theta),
                GateOp::Ry(k, theta) => reg.ry(*k, *theta),
                GateOp::Rz(k, theta) => reg.rz(*k, *theta),
                GateOp::Cnot(c, t) => reg.cnot(*c, *t),
                GateOp::Cz(a, b) => reg.cz(*a, *b),
                GateOp::Swap(a, b) => reg.swap(*a, *b),
                GateOp::CPhase(c, t, theta) => reg.controlled_phase(*c, *t, *theta),
                GateOp::Toffoli(c1, c2, t) => reg.toffoli(*c1, *c2, *t),
                GateOp::Fredkin(c, t1, t2) => reg.fredkin(*c, *t1, *t2),
                GateOp::Qft(ps) => reg.qft(ps),
                GateOp::InverseQft(ps) => reg.inverse_qft(ps),
                GateOp::Measure(k) => {
                    let m = reg.measure_position(*k, seed);
                    measurements.push((*k, m.value));
                    seed = seed.wrapping_add(1);
                }
                GateOp::MeasureAll => {
                    let m = reg.measure_all(seed);
                    for (i, &bit) in m.bits.iter().enumerate() {
                        measurements.push((i, bit));
                    }
                    seed = seed.wrapping_add(1);
                }
                GateOp::Barrier => {}
                GateOp::Custom(_, _) => {} // user would implement
            }
        }

        CircuitResult {
            measurements,
            register: reg,
            gate_count,
            depth,
        }
    }

    // ── ASCII Visualization ─────────────────────────────────────

    /// Render the circuit as ASCII art.
    pub fn to_ascii(&self) -> String {
        let mut lines: Vec<String> = (0..self.n)
            .map(|k| format!("  q{}: ", k))
            .collect();

        for op in &self.ops {
            match op {
                GateOp::Barrier => {
                    for line in &mut lines {
                        line.push_str("│ ");
                    }
                }

                GateOp::Cnot(c, t) => {
                    let min = (*c).min(*t);
                    let max = (*c).max(*t);
                    for k in 0..self.n {
                        if k == *c {
                            lines[k].push_str("──●──");
                        } else if k == *t {
                            lines[k].push_str("──X──");
                        } else if k > min && k < max {
                            lines[k].push_str("──│──");
                        } else {
                            lines[k].push_str("─────");
                        }
                    }
                }

                GateOp::Toffoli(c1, c2, t) => {
                    let min = (*c1).min(*c2).min(*t);
                    let max = (*c1).max(*c2).max(*t);
                    for k in 0..self.n {
                        if k == *c1 || k == *c2 {
                            lines[k].push_str("──●──");
                        } else if k == *t {
                            lines[k].push_str("──X──");
                        } else if k > min && k < max {
                            lines[k].push_str("──│──");
                        } else {
                            lines[k].push_str("─────");
                        }
                    }
                }

                GateOp::Swap(a, b) => {
                    let min = (*a).min(*b);
                    let max = (*a).max(*b);
                    for k in 0..self.n {
                        if k == *a || k == *b {
                            lines[k].push_str("──╳──");
                        } else if k > min && k < max {
                            lines[k].push_str("──│──");
                        } else {
                            lines[k].push_str("─────");
                        }
                    }
                }

                GateOp::Cz(a, b) => {
                    let min = (*a).min(*b);
                    let max = (*a).max(*b);
                    for k in 0..self.n {
                        if k == *a {
                            lines[k].push_str("──●──");
                        } else if k == *b {
                            lines[k].push_str("──Z──");
                        } else if k > min && k < max {
                            lines[k].push_str("──│──");
                        } else {
                            lines[k].push_str("─────");
                        }
                    }
                }

                _ => {
                    let lbl = op.label();
                    let positions = op.positions();
                    for k in 0..self.n {
                        if positions.contains(&k) {
                            let padded = format!("─[{}]─", lbl);
                            lines[k].push_str(&padded);
                        } else {
                            let width = lbl.len() + 4;
                            let dashes: String = "─".repeat(width);
                            lines[k].push_str(&dashes);
                        }
                    }
                }
            }
        }

        let mut result = format!("Circuit '{}' ({} positions, {} gates):\n", self.name, self.n, self.gate_count());
        for line in &lines {
            result.push_str(line);
            result.push('\n');
        }
        result
    }
}

impl fmt::Display for Circuit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_ascii())
    }
}

// ── Preset Circuits ─────────────────────────────────────────────────

/// Build a Bell pair circuit on positions 0 and 1.
pub fn bell_pair(n: usize) -> Circuit {
    Circuit::new(n, "Bell Pair")
        .h(0)
        .cnot(0, 1)
}

/// Build a GHZ state circuit on `n` positions.
pub fn ghz_state(n: usize) -> Circuit {
    let mut c = Circuit::new(n, "GHZ State").h(0);
    for k in 1..n {
        c = c.cnot(0, k);
    }
    c
}

/// Build a QFT circuit on all `n` positions.
pub fn qft_circuit(n: usize) -> Circuit {
    let positions: Vec<usize> = (0..n).collect();
    Circuit::new(n, "QFT").qft(&positions)
}

/// Build a Grover iteration circuit (oracle + diffusion).
/// The oracle is encoded via phase-flips on target states.
pub fn grover_iteration(n: usize, target: usize) -> Circuit {
    // This builds one iteration; the oracle marks `target` via X gates + multi-controlled Z
    // For simplicity we use the measure-free approach
    let mut c = Circuit::new(n, "Grover Iteration");

    // Oracle: flip phase of |target⟩
    // Apply X to positions where target bit is 0
    for k in 0..n {
        if (target >> (n - 1 - k)) & 1 == 0 {
            c = c.x(k);
        }
    }
    // Multi-controlled Z = H on last, Toffoli chain, H on last (for n≤3)
    if n == 2 {
        c = c.cz(0, 1);
    } else if n == 3 {
        c = c.h(2).toffoli(0, 1, 2).h(2);
    } else {
        // General case: just use CZ on first two as approximation
        // Full implementation would use ancillas
        c = c.cz(0, 1);
    }
    // Undo X gates
    for k in 0..n {
        if (target >> (n - 1 - k)) & 1 == 0 {
            c = c.x(k);
        }
    }

    // Diffusion
    c = c.h_all();
    for k in 0..n {
        c = c.x(k);
    }
    if n == 2 {
        c = c.cz(0, 1);
    } else if n == 3 {
        c = c.h(2).toffoli(0, 1, 2).h(2);
    } else {
        c = c.cz(0, 1);
    }
    for k in 0..n {
        c = c.x(k);
    }
    c.h_all()
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-8
    }

    fn prob(reg: &QuantumRegister, index: usize) -> f64 {
        let a = reg.amplitude(index);
        a.0 * a.0 + a.1 * a.1
    }

    #[test]
    fn test_bell_pair_circuit() {
        let result = bell_pair(2).execute();
        assert!(approx(prob(&result.register, 0b00), 0.5));
        assert!(approx(prob(&result.register, 0b11), 0.5));
        assert_eq!(result.gate_count, 2);
    }

    #[test]
    fn test_ghz_3() {
        let result = ghz_state(3).execute();
        assert!(approx(prob(&result.register, 0b000), 0.5));
        assert!(approx(prob(&result.register, 0b111), 0.5));
        assert_eq!(result.register.nonzero_count(), 2);
    }

    #[test]
    fn test_circuit_builder_chain() {
        let result = Circuit::new(2, "test")
            .h(0)
            .cnot(0, 1)
            .barrier()
            .measure_all()
            .execute();

        assert_eq!(result.measurements.len(), 2);
        let bits: Vec<u8> = result.measurements.iter().map(|m| m.1).collect();
        assert!(bits == vec![0, 0] || bits == vec![1, 1]);
    }

    #[test]
    fn test_circuit_h_all() {
        let result = Circuit::new(3, "test")
            .h_all()
            .execute();

        for i in 0..8 {
            assert!(approx(prob(&result.register, i), 0.125));
        }
    }

    #[test]
    fn test_circuit_repeat() {
        // H twice = identity
        let result = Circuit::new(1, "test")
            .repeat(2, |c| c.h(0))
            .execute();

        assert!(approx(prob(&result.register, 0), 1.0));
    }

    #[test]
    fn test_circuit_append() {
        let c1 = Circuit::new(2, "a").h(0);
        let c2 = Circuit::new(2, "b").cnot(0, 1);
        let result = c1.append(c2).execute();
        assert!(approx(prob(&result.register, 0b00), 0.5));
        assert!(approx(prob(&result.register, 0b11), 0.5));
    }

    #[test]
    fn test_circuit_qft() {
        let result = qft_circuit(2).execute();
        for i in 0..4 {
            assert!(approx(prob(&result.register, i), 0.25));
        }
    }

    #[test]
    fn test_circuit_toffoli() {
        let result = Circuit::new(3, "test")
            .x(0)
            .x(1)
            .toffoli(0, 1, 2)
            .execute();
        assert!(approx(prob(&result.register, 0b111), 1.0));
    }

    #[test]
    fn test_circuit_swap() {
        let result = Circuit::new(2, "test")
            .x(0) // |10⟩
            .swap(0, 1)
            .execute();
        assert!(approx(prob(&result.register, 0b01), 1.0));
    }

    #[test]
    fn test_ascii_rendering() {
        let c = Circuit::new(3, "Bell+Measure")
            .h(0)
            .cnot(0, 1)
            .barrier()
            .measure(0)
            .measure(1);

        let ascii = c.to_ascii();
        assert!(ascii.contains("Bell+Measure"));
        assert!(ascii.contains("q0:"));
        assert!(ascii.contains("q1:"));
        assert!(ascii.contains("q2:"));
    }

    #[test]
    fn test_circuit_gate_count() {
        let c = Circuit::new(2, "t")
            .h(0)
            .cnot(0, 1)
            .barrier()
            .measure_all();
        assert_eq!(c.gate_count(), 3); // h, cnot, measure_all (barrier excluded)
    }

    #[test]
    fn test_circuit_deterministic() {
        let r1 = Circuit::new(2, "t")
            .with_seed(42)
            .h(0)
            .cnot(0, 1)
            .measure_all()
            .execute();

        let r2 = Circuit::new(2, "t")
            .with_seed(42)
            .h(0)
            .cnot(0, 1)
            .measure_all()
            .execute();

        assert_eq!(r1.measurements, r2.measurements);
    }

    #[test]
    fn test_execute_on_existing_register() {
        let mut reg = QuantumRegister::new(2, "pre");
        reg.pauli_x(0); // |10⟩

        let result = Circuit::new(2, "add")
            .cnot(0, 1)
            .execute_on(reg);

        assert!(approx(prob(&result.register, 0b11), 1.0));
    }

    #[test]
    fn test_preset_grover_2qubit() {
        let c = Circuit::new(2, "Grover")
            .h_all()
            .append(grover_iteration(2, 0b11));

        let result = c.execute();
        assert!(prob(&result.register, 0b11) > 0.9);
    }

    #[test]
    fn test_circuit_with_rotations() {
        let result = Circuit::new(1, "rot")
            .rx(0, std::f64::consts::PI) // Rx(π) = -iX
            .execute();

        // Should flip |0⟩ to |1⟩ (with a global phase of -i)
        assert!(approx(prob(&result.register, 1), 1.0));
    }

    #[test]
    fn test_fredkin_circuit() {
        let result = Circuit::new(3, "fredkin")
            .x(0) // control = 1
            .x(1) // t1 = 1, t2 = 0
            .fredkin(0, 1, 2)
            .execute();
        // Should swap t1 and t2: |1,1,0⟩ → |1,0,1⟩
        assert!(approx(prob(&result.register, 0b101), 1.0));
    }
}
