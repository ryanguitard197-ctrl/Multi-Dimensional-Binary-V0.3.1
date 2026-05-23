//! # MDB-OS WebAssembly Bindings
//!
//! Run the full MDB quantum computing stack in any browser.
//! No install, no server, no quantum hardware — just load and go.
//!
//! ```js
//! import init, { WasmSuperBit, WasmRegister, WasmCircuit, shor, grover } from './mdb_wasm.js';
//! await init();
//!
//! // Dimensional cascade
//! const sb = new WasmSuperBit([1, 0, 1, 1, 0]);
//! console.log(sb.peek());   // JSON with all superposition states
//! console.log(sb.cascade(7)); // 7 dimensions of cascade
//!
//! // Quantum register
//! const reg = new WasmRegister(3, "bell");
//! reg.hadamard(0);
//! reg.cnot(0, 1);
//! console.log(reg.peek()); // Full state vector (non-destructive!)
//!
//! // Shor's algorithm
//! console.log(shor(15)); // { factors: [3, 5], ... }
//!
//! // Circuit model
//! const circ = WasmCircuit.bell(2);
//! console.log(circ.ascii());
//! console.log(circ.run());
//! ```

use wasm_bindgen::prelude::*;

// ═════════════════════════════════════════════════════════════════════
// Version
// ═════════════════════════════════════════════════════════════════════

/// Returns the MDB-OS version string.
#[wasm_bindgen]
pub fn version() -> String {
    format!("MDB-OS v{}", mdb_core::VERSION)
}

// ═════════════════════════════════════════════════════════════════════
// SuperBit
// ═════════════════════════════════════════════════════════════════════

/// A SuperBit — multidimensional binary data in superposition.
#[wasm_bindgen]
pub struct WasmSuperBit {
    inner: mdb_core::superbit::SuperBit,
}

#[wasm_bindgen]
impl WasmSuperBit {
    /// Create a SuperBit from a bit array (e.g., [1, 0, 1, 1]).
    #[wasm_bindgen(constructor)]
    pub fn new(bits: Vec<u8>) -> Self {
        Self {
            inner: mdb_core::superbit::SuperBit::from_bits(bits),
        }
    }

    /// Create a SuperBit from a binary string like "10110".
    #[wasm_bindgen(js_name = "fromString")]
    pub fn from_string(s: &str) -> Self {
        let bits: Vec<u8> = s
            .chars()
            .filter_map(|c| match c {
                '0' => Some(0),
                '1' => Some(1),
                _ => None,
            })
            .collect();
        Self {
            inner: mdb_core::superbit::SuperBit::from_bits(bits),
        }
    }

    /// Non-destructive peek at all superposition states (JSON).
    pub fn peek(&self) -> String {
        let view = self.inner.peek();
        serde_json::to_string_pretty(&PeekView::from(view)).unwrap_or_default()
    }

    /// Get the dimensional address (JSON).
    pub fn address(&self) -> String {
        let addr = self.inner.address();
        serde_json::to_string(&serde_json::json!({
            "n": addr.n,
            "d4_spacetime": addr.d4_spacetime,
            "d5_momentum": addr.d5_momentum,
        }))
        .unwrap_or_default()
    }

    /// Compute the dimensional cascade up to max_dim (JSON array of dimension vectors).
    pub fn cascade(&self, max_dim: usize) -> String {
        let c = mdb_core::coordinates::DimensionalCascade::from_bits(&self.inner.sigma, max_dim);
        let dim_names = [
            "D1_Value",
            "D2_Space",
            "D3_Time",
            "D4_Spacetime",
            "D5_Momentum",
            "D6_Energy",
        ];
        let dims: Vec<serde_json::Value> = c
            .dims
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let name = dim_names.get(i).unwrap_or(&"D?");
                serde_json::json!({ "name": name, "values": v })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "n": c.n,
            "dimensions": dims,
        }))
        .unwrap_or_default()
    }

    /// Number of bits.
    #[wasm_bindgen(js_name = "bitLength")]
    pub fn bit_length(&self) -> usize {
        self.inner.bit_length()
    }

    /// Number of superposition states.
    #[wasm_bindgen(js_name = "stateCount")]
    pub fn state_count(&self) -> usize {
        self.inner.state_count()
    }

    /// Fork (clone) the SuperBit.
    pub fn fork(&self) -> WasmSuperBit {
        WasmSuperBit {
            inner: self.inner.fork(),
        }
    }

    /// Add a new state to the superposition.
    #[wasm_bindgen(js_name = "addState")]
    pub fn add_state(&mut self, bits: Vec<u8>, label: &str, weight: f64) {
        self.inner.add_state(
            mdb_core::superbit::State {
                label: label.to_string(),
                pattern: bits,
            },
            weight,
        );
    }

    /// Collapse to a specific state by index. Returns JSON of the collapsed state.
    #[wasm_bindgen(js_name = "collapseTo")]
    pub fn collapse_to(&self, index: usize) -> String {
        match self.inner.collapse_to(index) {
            Ok(state) => serde_json::to_string(&serde_json::json!({
                "label": state.label,
                "pattern": state.pattern,
            }))
            .unwrap_or_default(),
            Err(e) => serde_json::to_string(&serde_json::json!({"error": format!("{:?}", e)}))
                .unwrap_or_default(),
        }
    }

    /// Get distances between all superposition states (JSON).
    #[wasm_bindgen(js_name = "stateDistances")]
    pub fn state_distances(&self) -> String {
        let dists = self.inner.state_distances();
        let items: Vec<serde_json::Value> = dists
            .iter()
            .map(|&(i, j, d)| serde_json::json!({"state_a": i, "state_b": j, "distance": d}))
            .collect();
        serde_json::to_string(&items).unwrap_or_default()
    }

    /// Serialize the SuperBit to bytes.
    pub fn encode(&self) -> Vec<u8> {
        self.inner.encode()
    }

    /// Deserialize a SuperBit from bytes.
    pub fn decode(data: &[u8]) -> Result<WasmSuperBit, JsError> {
        mdb_core::superbit::SuperBit::decode(data)
            .map(|inner| WasmSuperBit { inner })
            .map_err(|e| JsError::new(&format!("{:?}", e)))
    }
}

// Helper for SuperBit peek serialization
#[derive(serde::Serialize)]
struct PeekStateView {
    pattern: Vec<u8>,
    weight: f64,
    label: String,
    address: PeekAddress,
}

#[derive(serde::Serialize)]
struct PeekAddress {
    n: u64,
    d4_spacetime: f64,
    d5_momentum: u64,
}

#[derive(serde::Serialize)]
struct PeekView {
    states: Vec<PeekStateView>,
}

impl From<mdb_core::superbit::SuperpositionView> for PeekView {
    fn from(view: mdb_core::superbit::SuperpositionView) -> Self {
        PeekView {
            states: view
                .states
                .into_iter()
                .map(|s| PeekStateView {
                    pattern: s.pattern,
                    weight: s.weight,
                    label: s.label,
                    address: PeekAddress {
                        n: s.address.n,
                        d4_spacetime: s.address.d4_spacetime,
                        d5_momentum: s.address.d5_momentum,
                    },
                })
                .collect(),
        }
    }
}

// ═════════════════════════════════════════════════════════════════════
// Quantum Register
// ═════════════════════════════════════════════════════════════════════

/// A quantum register — full statevector simulator.
#[wasm_bindgen]
pub struct WasmRegister {
    inner: mdb_core::register::QuantumRegister,
}

#[wasm_bindgen]
impl WasmRegister {
    /// Create an n-qubit register initialized to |0...0⟩.
    #[wasm_bindgen(constructor)]
    pub fn new(n: usize, name: &str) -> Result<WasmRegister, JsError> {
        if n > 20 {
            return Err(JsError::new(
                "Max 20 qubits in WASM (1M amplitudes, ~16MB). Use native for larger.",
            ));
        }
        Ok(Self {
            inner: mdb_core::register::QuantumRegister::new(n, name),
        })
    }

    /// Create from integer value.
    #[wasm_bindgen(js_name = "fromInt")]
    pub fn from_int(n: usize, value: usize, name: &str) -> Result<WasmRegister, JsError> {
        if n > 20 {
            return Err(JsError::new("Max 20 qubits in WASM"));
        }
        Ok(Self {
            inner: mdb_core::register::QuantumRegister::from_int(n, value, name),
        })
    }

    /// Number of qubits.
    #[wasm_bindgen(getter)]
    pub fn n(&self) -> usize {
        self.inner.n
    }

    /// Dimension (2^n).
    pub fn dim(&self) -> usize {
        self.inner.dim()
    }

    // ── Single-qubit gates ───────────────────────────────────────────

    pub fn hadamard(&mut self, k: usize) {
        self.inner.hadamard(k);
    }

    #[wasm_bindgen(js_name = "pauliX")]
    pub fn pauli_x(&mut self, k: usize) {
        self.inner.pauli_x(k);
    }

    #[wasm_bindgen(js_name = "pauliY")]
    pub fn pauli_y(&mut self, k: usize) {
        self.inner.pauli_y(k);
    }

    #[wasm_bindgen(js_name = "pauliZ")]
    pub fn pauli_z(&mut self, k: usize) {
        self.inner.pauli_z(k);
    }

    pub fn phase(&mut self, k: usize, theta: f64) {
        self.inner.phase_gate(k, theta);
    }

    #[wasm_bindgen(js_name = "sGate")]
    pub fn s_gate(&mut self, k: usize) {
        self.inner.s_gate(k);
    }

    #[wasm_bindgen(js_name = "tGate")]
    pub fn t_gate(&mut self, k: usize) {
        self.inner.t_gate(k);
    }

    pub fn rx(&mut self, k: usize, theta: f64) {
        self.inner.rx(k, theta);
    }

    pub fn ry(&mut self, k: usize, theta: f64) {
        self.inner.ry(k, theta);
    }

    pub fn rz(&mut self, k: usize, theta: f64) {
        self.inner.rz(k, theta);
    }

    // ── Two-qubit gates ──────────────────────────────────────────────

    pub fn cnot(&mut self, control: usize, target: usize) {
        self.inner.cnot(control, target);
    }

    pub fn cz(&mut self, a: usize, b: usize) {
        self.inner.cz(a, b);
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        self.inner.swap(a, b);
    }

    #[wasm_bindgen(js_name = "controlledPhase")]
    pub fn controlled_phase(&mut self, control: usize, target: usize, theta: f64) {
        self.inner.controlled_phase(control, target, theta);
    }

    // ── Three-qubit gates ────────────────────────────────────────────

    pub fn toffoli(&mut self, c1: usize, c2: usize, target: usize) {
        self.inner.toffoli(c1, c2, target);
    }

    pub fn fredkin(&mut self, control: usize, t1: usize, t2: usize) {
        self.inner.fredkin(control, t1, t2);
    }

    // ── Composite operations ─────────────────────────────────────────

    pub fn qft(&mut self, positions: Vec<usize>) {
        self.inner.qft(&positions);
    }

    #[wasm_bindgen(js_name = "inverseQft")]
    pub fn inverse_qft(&mut self, positions: Vec<usize>) {
        self.inner.inverse_qft(&positions);
    }

    // ── Observation ──────────────────────────────────────────────────

    /// Non-destructive peek at the full state (JSON). MDB advantage.
    pub fn peek(&self) -> String {
        let view = self.inner.peek();
        let states: Vec<serde_json::Value> = view
            .states
            .iter()
            .filter(|s| s.probability > 1e-10)
            .map(|s| {
                let bits_str: String =
                    s.bits.iter().map(|b| if *b == 0 { '0' } else { '1' }).collect();
                serde_json::json!({
                    "bits": bits_str,
                    "probability": s.probability,
                    "phase": s.phase,
                    "index": s.index.to_string(),
                })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "n": self.inner.n,
            "nonzero_states": view.nonzero_states,
            "states": states,
        }))
        .unwrap_or_default()
    }

    /// Get probability distribution as a flat array.
    pub fn probabilities(&self) -> Vec<f64> {
        self.inner.probabilities()
    }

    /// Destructive measurement of all qubits.
    #[wasm_bindgen(js_name = "measureAll")]
    pub fn measure_all(&mut self, seed: u64) -> String {
        let result = self.inner.measure_all(seed);
        let bits_str: String = result
            .bits
            .iter()
            .map(|b| if *b == 0 { '0' } else { '1' })
            .collect();
        serde_json::to_string(&serde_json::json!({
            "bits": bits_str,
            "index": result.index.to_string(),
            "probability": result.probability,
        }))
        .unwrap_or_default()
    }

    /// Non-destructive sample (doesn't collapse state).
    #[wasm_bindgen(js_name = "sampleAll")]
    pub fn sample_all(&self, seed: u64) -> String {
        let result = self.inner.sample_all(seed);
        let bits_str: String = result
            .bits
            .iter()
            .map(|b| if *b == 0 { '0' } else { '1' })
            .collect();
        serde_json::to_string(&serde_json::json!({
            "bits": bits_str,
            "index": result.index.to_string(),
            "probability": result.probability,
        }))
        .unwrap_or_default()
    }

    /// Fork (clone) the register. Non-destructive — another MDB advantage.
    pub fn fork(&self) -> WasmRegister {
        WasmRegister {
            inner: self.inner.fork(),
        }
    }

    /// Reset to |0...0⟩.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Fidelity between two registers.
    pub fn fidelity(&self, other: &WasmRegister) -> f64 {
        self.inner.fidelity(&other.inner)
    }
}

// ═════════════════════════════════════════════════════════════════════
// Circuit
// ═════════════════════════════════════════════════════════════════════

/// A quantum circuit — declarative gate composition.
#[wasm_bindgen]
pub struct WasmCircuit {
    inner: mdb_core::circuit::Circuit,
}

#[wasm_bindgen]
impl WasmCircuit {
    /// Create an empty n-qubit circuit.
    #[wasm_bindgen(constructor)]
    pub fn new(n: usize, name: &str) -> Self {
        Self {
            inner: mdb_core::circuit::Circuit::new(n, name),
        }
    }

    /// Create a Bell pair circuit.
    pub fn bell(n: usize) -> WasmCircuit {
        WasmCircuit {
            inner: mdb_core::circuit::bell_pair(n),
        }
    }

    /// Create a GHZ state circuit.
    pub fn ghz(n: usize) -> WasmCircuit {
        WasmCircuit {
            inner: mdb_core::circuit::ghz_state(n),
        }
    }

    /// Create a QFT circuit.
    pub fn qft(n: usize) -> WasmCircuit {
        WasmCircuit {
            inner: mdb_core::circuit::qft_circuit(n),
        }
    }

    // Gate methods (each returns self for chaining in JS)
    pub fn h(mut self, k: usize) -> WasmCircuit {
        self.inner = self.inner.h(k);
        self
    }

    pub fn x(mut self, k: usize) -> WasmCircuit {
        self.inner = self.inner.x(k);
        self
    }

    pub fn y(mut self, k: usize) -> WasmCircuit {
        self.inner = self.inner.y(k);
        self
    }

    pub fn z(mut self, k: usize) -> WasmCircuit {
        self.inner = self.inner.z(k);
        self
    }

    #[wasm_bindgen(js_name = "sGate")]
    pub fn s_gate(mut self, k: usize) -> WasmCircuit {
        self.inner = self.inner.s(k);
        self
    }

    #[wasm_bindgen(js_name = "tGate")]
    pub fn t_gate(mut self, k: usize) -> WasmCircuit {
        self.inner = self.inner.t(k);
        self
    }

    #[wasm_bindgen(js_name = "cnot")]
    pub fn cnot(mut self, control: usize, target: usize) -> WasmCircuit {
        self.inner = self.inner.cnot(control, target);
        self
    }

    #[wasm_bindgen(js_name = "cz")]
    pub fn cz_gate(mut self, a: usize, b: usize) -> WasmCircuit {
        self.inner = self.inner.cz(a, b);
        self
    }

    #[wasm_bindgen(js_name = "swap")]
    pub fn swap_gate(mut self, a: usize, b: usize) -> WasmCircuit {
        self.inner = self.inner.swap(a, b);
        self
    }

    #[wasm_bindgen(js_name = "toffoli")]
    pub fn toffoli_gate(mut self, c1: usize, c2: usize, target: usize) -> WasmCircuit {
        self.inner = self.inner.toffoli(c1, c2, target);
        self
    }

    #[wasm_bindgen(js_name = "measureAll")]
    pub fn measure_all(mut self) -> WasmCircuit {
        self.inner = self.inner.measure_all();
        self
    }

    #[wasm_bindgen(js_name = "hAll")]
    pub fn h_all(mut self) -> WasmCircuit {
        self.inner = self.inner.h_all();
        self
    }

    /// ASCII visualization of the circuit.
    pub fn ascii(&self) -> String {
        self.inner.to_ascii()
    }

    /// Gate count.
    #[wasm_bindgen(js_name = "gateCount")]
    pub fn gate_count(&self) -> usize {
        self.inner.gate_count()
    }

    /// Circuit depth.
    pub fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Execute the circuit and return results (JSON).
    pub fn run(self) -> String {
        let result = self.inner.execute();
        let measurements: Vec<serde_json::Value> = result
            .measurements
            .iter()
            .map(|&(pos, val)| serde_json::json!({"position": pos, "value": val}))
            .collect();

        let peek = result.register.peek();
        let states: Vec<serde_json::Value> = peek
            .states
            .iter()
            .filter(|s| s.probability > 1e-10)
            .map(|s| {
                let bits_str: String =
                    s.bits.iter().map(|b| if *b == 0 { '0' } else { '1' }).collect();
                serde_json::json!({
                    "bits": bits_str,
                    "probability": s.probability,
                    "phase": s.phase,
                })
            })
            .collect();

        serde_json::to_string_pretty(&serde_json::json!({
            "gate_count": result.gate_count,
            "depth": result.depth,
            "measurements": measurements,
            "final_state": states,
        }))
        .unwrap_or_default()
    }
}

// ═════════════════════════════════════════════════════════════════════
// Algorithms (standalone functions)
// ═════════════════════════════════════════════════════════════════════

/// Shor's factoring algorithm. Returns JSON with factors, or null if can't factor.
#[wasm_bindgen]
pub fn shor(n: u64) -> String {
    match mdb_core::algorithms::shors_factor(n) {
        Some(r) => serde_json::to_string(&serde_json::json!({
            "number": n,
            "factors": [r.factors.0, r.factors.1],
            "period": r.period,
            "attempts": r.attempts,
        }))
        .unwrap_or_default(),
        None => serde_json::to_string(&serde_json::json!({
            "number": n,
            "error": "Cannot factor (prime or too large)",
        }))
        .unwrap_or_default(),
    }
}

/// Grover's search. Finds target in 2^n_qubits search space. Returns JSON.
#[wasm_bindgen]
pub fn grover(n_qubits: usize, target: usize) -> String {
    let target_bits: Vec<u8> = (0..n_qubits)
        .map(|k| ((target >> (n_qubits - 1 - k)) & 1) as u8)
        .collect();
    let result =
        mdb_core::algorithms::grovers_search(n_qubits, &|bits: &[u8]| bits == target_bits.as_slice(), None);
    serde_json::to_string(&serde_json::json!({
        "search_space": 1u64 << n_qubits,
        "target": target,
        "found": result.index.to_string(),
        "correct": result.index == target as u64,
        "iterations": result.iterations,
        "probability": result.probability,
    }))
    .unwrap_or_default()
}

/// Deutsch-Jozsa algorithm on a parity function. Returns "Constant" or "Balanced".
#[wasm_bindgen(js_name = "deutschJozsa")]
pub fn deutsch_jozsa(n_qubits: usize) -> String {
    let result = mdb_core::algorithms::deutsch_jozsa(n_qubits, &|x: usize| {
        let mut p = 0u8;
        let mut v = x;
        while v > 0 {
            p ^= (v & 1) as u8;
            v >>= 1;
        }
        p
    });
    serde_json::to_string(&serde_json::json!({
        "n_qubits": n_qubits,
        "result": format!("{:?}", result),
        "classical_queries_needed": (1u64 << n_qubits) / 2 + 1,
        "quantum_queries_used": 1,
    }))
    .unwrap_or_default()
}

/// Quantum teleportation. Teleports state alpha|0⟩ + beta|1⟩.
/// Takes alpha_re, alpha_im, beta_re, beta_im.
#[wasm_bindgen]
pub fn teleport(alpha_re: f64, alpha_im: f64, beta_re: f64, beta_im: f64, seed: u64) -> String {
    let result = mdb_core::algorithms::quantum_teleport(
        (alpha_re, alpha_im),
        (beta_re, beta_im),
        seed,
    );
    serde_json::to_string_pretty(&serde_json::json!({
        "input_alpha": [alpha_re, alpha_im],
        "input_beta": [beta_re, beta_im],
        "fidelity": result.fidelity,
        "alice_bits": [result.alice_bits.0, result.alice_bits.1],
    }))
    .unwrap_or_default()
}

/// Compute a dimensional cascade for a binary string. Returns JSON.
#[wasm_bindgen(js_name = "dimensionalCascade")]
pub fn dimensional_cascade(bits_str: &str, max_dim: usize) -> String {
    let bits: Vec<u8> = bits_str
        .chars()
        .filter_map(|c| match c {
            '0' => Some(0),
            '1' => Some(1),
            _ => None,
        })
        .collect();
    if bits.is_empty() {
        return "{}".to_string();
    }

    let c = mdb_core::coordinates::DimensionalCascade::from_bits(&bits, max_dim);
    let dim_names = [
        "D1_Value",
        "D2_Space",
        "D3_Time",
        "D4_Spacetime",
        "D5_Momentum",
        "D6_Energy",
    ];
    let dims: Vec<serde_json::Value> = c
        .dims
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let name = dim_names
                .get(i)
                .unwrap_or(&"D?")
                .to_string();
            serde_json::json!({ "name": name, "dimension": i + 1, "values": v })
        })
        .collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "bits": bits,
        "n": c.n,
        "dimensions": dims,
    }))
    .unwrap_or_default()
}

/// Run the benchmark suite and return a text report.
#[wasm_bindgen(js_name = "runBenchmarks")]
pub fn run_benchmarks() -> String {
    mdb_core::benchmarks::report()
}

// ═════════════════════════════════════════════════════════════════════
// Sparse Quantum Register — THE CORE INNOVATION
// ═════════════════════════════════════════════════════════════════════
//
// Unlike WasmRegister (dense, capped at 20 qubits), WasmSparseRegister
// has NO position limit.  Memory scales with populated states, not 2^n.
// A 64-position GHZ state uses ~200 bytes, not 295 exabytes.

/// A sparse quantum register — stores only populated basis states.
/// No position limit.  Memory = O(populated states).
#[wasm_bindgen]
pub struct WasmSparseRegister {
    inner: mdb_core::sparse_register::SparseQuantumRegister,
}

#[wasm_bindgen]
impl WasmSparseRegister {
    /// Create an n-position sparse register initialized to |0…0⟩.
    /// No upper limit on n — memory scales with populated states, not 2^n.
    #[wasm_bindgen(constructor)]
    pub fn new(n: usize, name: &str) -> Result<WasmSparseRegister, JsError> {
        if n == 0 {
            return Err(JsError::new("Register width must be >= 1"));
        }
        Ok(Self {
            inner: mdb_core::sparse_register::SparseQuantumRegister::new(n, name),
        })
    }

    /// Create from an integer basis state |value⟩.
    #[wasm_bindgen(js_name = "fromInt")]
    pub fn from_int(n: usize, value: u64, name: &str) -> Result<WasmSparseRegister, JsError> {
        if n == 0 {
            return Err(JsError::new("Register width must be >= 1"));
        }
        Ok(Self {
            inner: mdb_core::sparse_register::SparseQuantumRegister::from_int(n, value, name),
        })
    }

    /// Create from a bit string (e.g. [1, 0, 1, 1]).
    #[wasm_bindgen(js_name = "fromBits")]
    pub fn from_bits(bits: Vec<u8>, name: &str) -> WasmSparseRegister {
        Self {
            inner: mdb_core::sparse_register::SparseQuantumRegister::from_bits(&bits, name),
        }
    }

    // ── Properties ───────────────────────────────────────────────────

    /// Number of positions (qubits).
    #[wasm_bindgen(getter)]
    pub fn n(&self) -> usize {
        self.inner.n
    }

    /// Number of currently populated basis states.
    pub fn population(&self) -> usize {
        self.inner.population()
    }

    /// Full Hilbert space dimension 2^n (as string — can exceed u64).
    #[wasm_bindgen(js_name = "hilbertDim")]
    pub fn hilbert_dim(&self) -> String {
        self.inner.hilbert_dim().to_string()
    }

    /// Sparsity ratio: populated / 2^n.
    pub fn sparsity(&self) -> f64 {
        self.inner.sparsity()
    }

    /// Memory usage in bytes.
    #[wasm_bindgen(js_name = "memoryBytes")]
    pub fn memory_bytes(&self) -> usize {
        self.inner.memory_bytes()
    }

    /// Total probability (should be ≈ 1.0 for normalized state).
    #[wasm_bindgen(js_name = "totalProbability")]
    pub fn total_probability(&self) -> f64 {
        self.inner.total_probability()
    }

    // ── Single-qubit gates ───────────────────────────────────────────

    pub fn hadamard(&mut self, k: usize) {
        self.inner.hadamard(k);
    }

    #[wasm_bindgen(js_name = "pauliX")]
    pub fn pauli_x(&mut self, k: usize) {
        self.inner.pauli_x(k);
    }

    #[wasm_bindgen(js_name = "pauliY")]
    pub fn pauli_y(&mut self, k: usize) {
        self.inner.pauli_y(k);
    }

    #[wasm_bindgen(js_name = "pauliZ")]
    pub fn pauli_z(&mut self, k: usize) {
        self.inner.pauli_z(k);
    }

    pub fn phase(&mut self, k: usize, theta: f64) {
        self.inner.phase_gate(k, theta);
    }

    #[wasm_bindgen(js_name = "sGate")]
    pub fn s_gate(&mut self, k: usize) {
        self.inner.s_gate(k);
    }

    #[wasm_bindgen(js_name = "tGate")]
    pub fn t_gate(&mut self, k: usize) {
        self.inner.t_gate(k);
    }

    pub fn rx(&mut self, k: usize, theta: f64) {
        self.inner.rx(k, theta);
    }

    pub fn ry(&mut self, k: usize, theta: f64) {
        self.inner.ry(k, theta);
    }

    pub fn rz(&mut self, k: usize, theta: f64) {
        self.inner.rz(k, theta);
    }

    // ── Two-qubit gates ──────────────────────────────────────────────

    pub fn cnot(&mut self, control: usize, target: usize) {
        self.inner.cnot(control, target);
    }

    pub fn cz(&mut self, a: usize, b: usize) {
        self.inner.cz(a, b);
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        self.inner.swap(a, b);
    }

    #[wasm_bindgen(js_name = "controlledPhase")]
    pub fn controlled_phase(&mut self, control: usize, target: usize, theta: f64) {
        self.inner.controlled_phase(control, target, theta);
    }

    // ── Three-qubit gates ────────────────────────────────────────────

    pub fn toffoli(&mut self, c1: usize, c2: usize, target: usize) {
        self.inner.toffoli(c1, c2, target);
    }

    pub fn fredkin(&mut self, control: usize, t1: usize, t2: usize) {
        self.inner.fredkin(control, t1, t2);
    }

    // ── Composite operations ─────────────────────────────────────────

    pub fn qft(&mut self, positions: Vec<usize>) {
        self.inner.qft(&positions);
    }

    #[wasm_bindgen(js_name = "inverseQft")]
    pub fn inverse_qft(&mut self, positions: Vec<usize>) {
        self.inner.inverse_qft(&positions);
    }

    /// Grover diffusion operator.
    #[wasm_bindgen(js_name = "groverDiffusion")]
    pub fn grover_diffusion(&mut self) {
        self.inner.grover_diffusion();
    }

    // ── Observation (MDB advantages) ─────────────────────────────────

    /// peek() — Non-destructive full state readout.
    /// Impossible on quantum hardware (violates projection postulate).
    /// Returns JSON with all populated states, probabilities, and phases.
    pub fn peek(&self) -> String {
        let view = self.inner.peek();
        let states: Vec<serde_json::Value> = view
            .states
            .iter()
            .filter(|s| s.probability > 1e-15)
            .map(|s| {
                let bits_str: String =
                    s.bits.iter().map(|b| if *b == 0 { '0' } else { '1' }).collect();
                serde_json::json!({
                    "bits": bits_str,
                    "probability": s.probability,
                    "phase": s.phase,
                    "index": s.index.to_string(),
                })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "n": self.inner.n,
            "populated_states": view.nonzero_states,
            "memory_bytes": self.inner.memory_bytes(),
            "sparsity": self.inner.sparsity(),
            "states": states,
        }))
        .unwrap_or_default()
    }

    /// fork() — Independent deep copy of the register.
    /// Impossible on quantum hardware (violates no-cloning theorem).
    /// Modifying the fork cannot affect the original.
    pub fn fork(&self) -> WasmSparseRegister {
        WasmSparseRegister {
            inner: self.inner.fork(),
        }
    }

    /// Fidelity (overlap) between this register and another.
    /// Returns |⟨ψ|φ⟩|² — 1.0 means identical states, 0.0 means orthogonal.
    pub fn fidelity(&self, other: &WasmSparseRegister) -> f64 {
        self.inner.fidelity(&other.inner)
    }

    /// Non-destructive sample — picks one outcome without collapsing state.
    #[wasm_bindgen(js_name = "sampleAll")]
    pub fn sample_all(&self, seed: u64) -> String {
        let result = self.inner.sample_all(seed);
        let bits_str: String = result
            .bits
            .iter()
            .map(|b| if *b == 0 { '0' } else { '1' })
            .collect();
        serde_json::to_string(&serde_json::json!({
            "bits": bits_str,
            "index": result.index.to_string(),
            "probability": result.probability,
        }))
        .unwrap_or_default()
    }

    /// Destructive measurement of all positions.
    #[wasm_bindgen(js_name = "measureAll")]
    pub fn measure_all(&mut self, seed: u64) -> String {
        let result = self.inner.measure_all(seed);
        let bits_str: String = result
            .bits
            .iter()
            .map(|b| if *b == 0 { '0' } else { '1' })
            .collect();
        serde_json::to_string(&serde_json::json!({
            "bits": bits_str,
            "index": result.index.to_string(),
            "probability": result.probability,
        }))
        .unwrap_or_default()
    }

    /// Measure a single position (partial collapse).
    #[wasm_bindgen(js_name = "measurePosition")]
    pub fn measure_position(&mut self, k: usize, seed: u64) -> String {
        let result = self.inner.measure_position(k, seed);
        serde_json::to_string(&serde_json::json!({
            "position": result.position,
            "value": result.value,
            "probability": result.probability,
        }))
        .unwrap_or_default()
    }

    /// Prune states below threshold. Returns number of states removed.
    pub fn prune(&mut self, threshold: f64) -> usize {
        self.inner.prune(threshold)
    }

    /// Auto-prune using the register's default threshold.
    #[wasm_bindgen(js_name = "autoPrune")]
    pub fn auto_prune(&mut self) -> usize {
        self.inner.auto_prune()
    }

    /// Normalize amplitudes so total probability = 1.
    pub fn normalize(&mut self) {
        self.inner.normalize();
    }

    /// Reset to |0…0⟩.
    pub fn reset(&mut self) {
        // Re-create: cheapest way to reset a sparse register
        self.inner = mdb_core::sparse_register::SparseQuantumRegister::new(
            self.inner.n,
            &self.inner.name,
        );
    }

    /// Get the dimensional cascade snapshot — maps each populated state
    /// through the Fibonacci coordinate system.
    #[wasm_bindgen(js_name = "cascadeSnapshot")]
    pub fn cascade_snapshot(&self) -> String {
        let entries = self.inner.cascade_snapshot();
        let data: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                let bits_str: String =
                    e.bits.iter().map(|b| if *b == 0 { '0' } else { '1' }).collect();
                serde_json::json!({
                    "index": e.index.to_string(),
                    "bits": bits_str,
                    "probability": e.probability,
                    "address": {
                        "n": e.address.n,
                        "d4_spacetime": e.address.d4_spacetime,
                        "d5_momentum": e.address.d5_momentum,
                    }
                })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "n": self.inner.n,
            "populated_states": self.inner.population(),
            "entries": data,
        }))
        .unwrap_or_default()
    }

    /// Probability distribution as JSON (only populated states).
    pub fn probabilities(&self) -> String {
        let probs = self.inner.probabilities();
        let data: Vec<serde_json::Value> = probs
            .iter()
            .map(|(idx, p)| serde_json::json!({"index": idx.to_string(), "probability": p}))
            .collect();
        serde_json::to_string(&data).unwrap_or_default()
    }
}
