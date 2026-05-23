//! # MDB Core — Multidimensional Binary Computing Engine
//!
//! The canonical Rust implementation of the MDB paradigm.
//!
//! MDB redefines the atomic unit of computation from a classical bit (0 or 1)
//! to a **SuperBit** — a binary string that exists as a geometric object in
//! abstract higher-dimensional coordinate space.  Binary data is no longer flat;
//! it carries intrinsic dimensional properties computed via a recursive cascade
//! where each dimension derives from the two below it (Fibonacci addition).
//!
//! ## Dimensional Cascade
//!
//! | Dim | Name       | Description                                          |
//! |-----|------------|------------------------------------------------------|
//! | D1  | Value      | Per-position bit weight (0.3 for 1, 0.2 for 0)      |
//! | D2  | Space      | Length-scaled weight per position                    |
//! | D3  | Time       | 1:1 from Space (simulated time)                     |
//! | D4  | Spacetime  | D2 + D3 — the first combined dimension               |
//! | D5  | Momentum   | D3 + D4 — how things move through spacetime          |
//! | D6  | Energy     | D4 + D5 — emerges from momentum in spacetime         |
//! | D7+ | (recurse)  | D(k) = D(k-2) + D(k-1)                              |
//!
//! The cascade produces Fibonacci coefficients (1,1,2,3,5,8,…) and the ratio
//! D(k)/D(k-1) converges to the Golden Ratio φ ≈ 1.618.
//!
//! ## Non-Destructive Superposition
//!
//! The SuperBit holds multiple states in superposition with probability
//! weights.  Unlike a physical qubit, a SuperBit can be **read without
//! collapsing**.  You can:
//!
//! - `peek()` — inspect every state, weight, and dimensional address
//! - `fork()` — create independent clones for parallel exploration
//! - `collapse_to(i)` — choose a specific state without modifying σ
//! - `state_distances()` — compare states dimensionally
//! - `evolve_cascade_preview()` — see what evolution *would* do
//!
//! No decoherence.  No measurement problem.  No no-cloning theorem.
//!
//! ## Modules
//!
//! - [`coordinates`] — Dimensional cascade engine and address computation
//! - [`superbit`] — The SuperBit atomic unit with non-destructive superposition
//! - [`definitions`] — Immutable anchor positions (DefinitionsList)
//! - [`fold`] — Lossless geometric folding engine
//! - [`unfold`] — Lossless geometric unfolding (inverse of fold)
//! - [`index`] — DimensionalIndex for O(1) guaranteed retrieval
//! - [`network`] — MDBNetwork with cascade-aware entanglement
//! - [`evolution`] — Dimensional, learning, and cascade (φ-driven) evolution
//! - [`gates`] — Dimensional gate system (Hadamard, CNOT, Phase, Oracle)
//! - [`search`] — Superposition search algorithms (dimensional, pattern, fitness)
//! - [`register`] — Quantum register (full statevector, all standard gates, QFT)
//! - [`sparse_register`] — Cascade-addressed sparse quantum register (scales past 24 qubits)
//! - [`algorithms`] — Quantum algorithms (Shor's, Grover's, Deutsch–Jozsa, teleportation)
//! - [`circuit`] — Declarative circuit builder with ASCII visualization
//! - [`error_correction`] — Quantum error correction (bit-flip, phase-flip, Shor 9-qubit, Steane 7-qubit)
//! - [`phase_estimation`] — Quantum phase estimation (QPE)
//! - [`variational`] — Variational algorithms (VQE, QAOA)

pub mod algorithms;
pub mod circuit;
pub mod coordinates;
pub mod definitions;
pub mod evolution;
pub mod fold;
pub mod gates;
pub mod index;
pub mod network;
pub mod register;
pub mod sparse_register;
pub mod benchmarks;
pub mod error_correction;
pub mod persistence;
pub mod phase_estimation;
pub mod search;
pub mod superbit;
pub mod unfold;
pub mod security;
pub mod variational;

/// The Golden Ratio — emerges naturally from the dimensional cascade.
/// D(k)/D(k-1) converges to this value as k increases.
pub const PHI: f64 = 1.618_033_988_749_895;

/// MDB version string.
pub const VERSION: &str = "0.3.0";
