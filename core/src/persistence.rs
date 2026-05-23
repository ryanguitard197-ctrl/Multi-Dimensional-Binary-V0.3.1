//! # Persistence — Save / Load MDB state to disk
//!
//! Serialises SuperBits, QuantumRegisters, and circuits to a compact
//! JSON-based format (.mdb files).  Supports:
//!
//! - Single SuperBit save/load
//! - Batch export of multiple SuperBits
//! - Quantum register state snapshots
//! - Full workspace save (multiple named SuperBits + registers)
//!
//! MDB advantage: because superposition is non-destructive, we can serialise
//! the *full* quantum state — something physically impossible with real qubits.

use crate::coordinates::DimensionalCascade;
use crate::register::QuantumRegister;
use crate::superbit::SuperBit;
use serde::{Deserialize, Serialize};

// ═════════════════════════════════════════════════════════════════════
// Serialisable types
// ═════════════════════════════════════════════════════════════════════

/// Serialisable representation of a SuperBit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperBitSnapshot {
    /// Human-readable label.
    pub label: String,
    /// The sigma (raw bits).
    pub sigma: Vec<u8>,
    /// Superposition state patterns.
    pub state_patterns: Vec<Vec<u8>>,
    /// State labels.
    pub state_labels: Vec<String>,
    /// Probability weights for each state.
    pub weights: Vec<f64>,
    /// Dimensional cascade of sigma: dims[k] is D(k+1) as Vec<f64>.
    pub cascade_dims: Vec<Vec<f64>>,
    /// Bit count.
    pub n: usize,
}

/// Serialisable quantum register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSnapshot {
    pub label: String,
    pub n: usize,
    /// Amplitudes as (index, re, im) triples. Only non-zero entries stored (sparse).
    pub amplitudes: Vec<(usize, f64, f64)>,
}

/// A full workspace snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Format version.
    pub version: String,
    /// SuperBit collection.
    pub superbits: Vec<SuperBitSnapshot>,
    /// Register collection.
    pub registers: Vec<RegisterSnapshot>,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
}

// ═════════════════════════════════════════════════════════════════════
// Snapshot creation
// ═════════════════════════════════════════════════════════════════════

impl SuperBitSnapshot {
    /// Create a snapshot from a SuperBit.
    pub fn from_superbit(sb: &SuperBit, label: &str) -> Self {
        let view = sb.peek();

        let state_patterns: Vec<Vec<u8>> = view.states.iter().map(|s| s.pattern.clone()).collect();
        let state_labels: Vec<String> = view.states.iter().map(|s| s.label.clone()).collect();
        let weights: Vec<f64> = view.states.iter().map(|s| s.weight).collect();

        // Capture cascade of sigma
        let c = DimensionalCascade::from_bits(&sb.sigma, 6);

        Self {
            label: label.to_string(),
            sigma: sb.sigma.clone(),
            state_patterns,
            state_labels,
            weights,
            cascade_dims: c.dims.clone(),
            n: c.n,
        }
    }

    /// Reconstruct a SuperBit from a snapshot.
    pub fn to_superbit(&self) -> SuperBit {
        let mut sb = SuperBit::from_bits(self.sigma.clone());
        // If there are multiple states, add them
        for i in 1..self.state_patterns.len() {
            let state = crate::superbit::State {
                label: self.state_labels.get(i).cloned().unwrap_or_else(|| format!("state_{}", i)),
                pattern: self.state_patterns[i].clone(),
            };
            let w = self.weights.get(i).copied().unwrap_or(0.0);
            if w > 0.0 {
                sb.add_state(state, w);
            }
        }
        sb
    }
}

impl RegisterSnapshot {
    /// Create a snapshot from a QuantumRegister (via peek — non-destructive).
    pub fn from_register(reg: &QuantumRegister, label: &str) -> Self {
        let dim = reg.dim();
        let mut amplitudes = Vec::new();

        for i in 0..dim {
            let amp = reg.amplitude(i);
            if amp.0.abs() > 1e-15 || amp.1.abs() > 1e-15 {
                amplitudes.push((i, amp.0, amp.1));
            }
        }

        Self {
            label: label.to_string(),
            n: reg.n,
            amplitudes,
        }
    }

    /// Reconstruct a QuantumRegister from a snapshot.
    pub fn to_register(&self) -> QuantumRegister {
        let mut reg = QuantumRegister::new(self.n, &self.label);
        // Clear default |0...0⟩ state
        let dim = reg.dim();
        for i in 0..dim {
            reg.set_amplitude(i, (0.0, 0.0));
        }
        // Restore saved amplitudes
        for &(idx, re, im) in &self.amplitudes {
            reg.set_amplitude(idx, (re, im));
        }
        reg
    }
}

impl Workspace {
    /// Create an empty workspace.
    pub fn new() -> Self {
        Self {
            version: crate::VERSION.to_string(),
            superbits: Vec::new(),
            registers: Vec::new(),
            metadata: None,
        }
    }

    /// Add a SuperBit to the workspace.
    pub fn add_superbit(&mut self, sb: &SuperBit, label: &str) {
        self.superbits.push(SuperBitSnapshot::from_superbit(sb, label));
    }

    /// Add a QuantumRegister to the workspace.
    pub fn add_register(&mut self, reg: &QuantumRegister, label: &str) {
        self.registers.push(RegisterSnapshot::from_register(reg, label));
    }

    /// Serialise to JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("workspace serialisation failed")
    }

    /// Serialise to compact JSON (no whitespace).
    pub fn to_json_compact(&self) -> String {
        serde_json::to_string(self).expect("workspace serialisation failed")
    }

    /// Deserialise from JSON string.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("parse error: {}", e))
    }

    /// Save to a file.
    pub fn save(&self, path: &str) -> Result<(), String> {
        let json = self.to_json();
        std::fs::write(path, json).map_err(|e| format!("write error: {}", e))
    }

    /// Load from a file.
    pub fn load(path: &str) -> Result<Self, String> {
        let json = std::fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
        Self::from_json(&json)
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

// ═════════════════════════════════════════════════════════════════════
// Quick save/load helpers
// ═════════════════════════════════════════════════════════════════════

/// Save a single SuperBit to a .mdb file.
pub fn save_superbit(sb: &SuperBit, label: &str, path: &str) -> Result<(), String> {
    let mut ws = Workspace::new();
    ws.add_superbit(sb, label);
    ws.save(path)
}

/// Load the first SuperBit from a .mdb file.
pub fn load_superbit(path: &str) -> Result<SuperBit, String> {
    let ws = Workspace::load(path)?;
    ws.superbits
        .first()
        .map(|s| s.to_superbit())
        .ok_or_else(|| "no superbits in file".to_string())
}

/// Save a QuantumRegister to a .mdb file.
pub fn save_register(reg: &QuantumRegister, label: &str, path: &str) -> Result<(), String> {
    let mut ws = Workspace::new();
    ws.add_register(reg, label);
    ws.save(path)
}

/// Load the first QuantumRegister from a .mdb file.
pub fn load_register(path: &str) -> Result<QuantumRegister, String> {
    let ws = Workspace::load(path)?;
    ws.registers
        .first()
        .map(|r| r.to_register())
        .ok_or_else(|| "no registers in file".to_string())
}

// ═════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_superbit_roundtrip() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 1]);
        let snap = SuperBitSnapshot::from_superbit(&sb, "test");
        let json = serde_json::to_string(&snap).unwrap();
        let restored: SuperBitSnapshot = serde_json::from_str(&json).unwrap();
        let sb2 = restored.to_superbit();
        let v1 = sb.peek();
        let v2 = sb2.peek();
        assert_eq!(v1.states.len(), v2.states.len());
        assert_eq!(v1.states[0].pattern, v2.states[0].pattern);
    }

    #[test]
    fn test_register_roundtrip() {
        let mut reg = QuantumRegister::new(3, "test");
        reg.hadamard(0);
        reg.cnot(0, 1);

        let snap = RegisterSnapshot::from_register(&reg, "bell");
        let json = serde_json::to_string(&snap).unwrap();
        let restored: RegisterSnapshot = serde_json::from_str(&json).unwrap();
        let reg2 = restored.to_register();

        let fid = reg.fidelity(&reg2);
        assert!(fid > 0.999, "fidelity should be ~1.0, got {}", fid);
    }

    #[test]
    fn test_register_sparse_storage() {
        let mut reg = QuantumRegister::new(3, "ghz");
        reg.hadamard(0);
        reg.cnot(0, 1);
        reg.cnot(1, 2);

        let snap = RegisterSnapshot::from_register(&reg, "ghz");
        assert!(
            snap.amplitudes.len() <= 4,
            "GHZ should be sparse, got {} amplitudes",
            snap.amplitudes.len()
        );
    }

    #[test]
    fn test_workspace_roundtrip() {
        let sb1 = SuperBit::from_bits(vec![1, 0, 1]);
        let sb2 = SuperBit::from_bits(vec![0, 1, 0]);

        let mut reg = QuantumRegister::new(2, "epr");
        reg.hadamard(0);
        reg.cnot(0, 1);

        let mut ws = Workspace::new();
        ws.add_superbit(&sb1, "alpha");
        ws.add_superbit(&sb2, "beta");
        ws.add_register(&reg, "entangled");

        let json = ws.to_json();
        let ws2 = Workspace::from_json(&json).unwrap();

        assert_eq!(ws2.superbits.len(), 2);
        assert_eq!(ws2.registers.len(), 1);
        assert_eq!(ws2.superbits[0].label, "alpha");
        assert_eq!(ws2.superbits[1].label, "beta");
        assert_eq!(ws2.registers[0].label, "entangled");
    }

    #[test]
    fn test_save_load_file() {
        let sb = SuperBit::from_bits(vec![1, 1, 0, 0, 1]);
        let path = "/tmp/test_mdb_save.mdb";
        save_superbit(&sb, "test_save", path).unwrap();

        let loaded = load_superbit(path).unwrap();
        let v1 = sb.peek();
        let v2 = loaded.peek();
        assert_eq!(v1.states[0].pattern, v2.states[0].pattern);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_save_load_register_file() {
        let mut reg = QuantumRegister::new(4, "test");
        reg.hadamard(0);
        reg.hadamard(1);
        reg.cnot(0, 2);
        reg.cnot(1, 3);

        let path = "/tmp/test_mdb_reg.mdb";
        save_register(&reg, "entangled_pair", path).unwrap();

        let loaded = load_register(path).unwrap();
        let fid = reg.fidelity(&loaded);
        assert!(fid > 0.999, "fidelity {}", fid);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_cascade_in_snapshot() {
        let sb = SuperBit::from_bits(vec![1, 0, 1, 1, 0]);
        let snap = SuperBitSnapshot::from_superbit(&sb, "cascade_test");
        assert_eq!(snap.n, 5);
        assert!(snap.cascade_dims.len() >= 6); // D1 through D6
        assert_eq!(snap.cascade_dims[0].len(), 5); // D1 has 5 elements
    }

    #[test]
    fn test_workspace_metadata() {
        let mut ws = Workspace::new();
        ws.metadata = Some(serde_json::json!({
            "author": "Ryan Guitard",
            "experiment": "Bell state teleportation",
            "date": "2026-05-17"
        }));

        let json = ws.to_json();
        let ws2 = Workspace::from_json(&json).unwrap();
        let meta = ws2.metadata.unwrap();
        assert_eq!(meta["author"], "Ryan Guitard");
    }

    #[test]
    fn test_empty_workspace() {
        let ws = Workspace::new();
        let json = ws.to_json();
        let ws2 = Workspace::from_json(&json).unwrap();
        assert_eq!(ws2.superbits.len(), 0);
        assert_eq!(ws2.registers.len(), 0);
    }

    #[test]
    fn test_compact_json() {
        let mut ws = Workspace::new();
        let sb = SuperBit::from_bits(vec![1, 0]);
        ws.add_superbit(&sb, "tiny");

        let pretty = ws.to_json();
        let compact = ws.to_json_compact();
        assert!(compact.len() < pretty.len());
    }
}
