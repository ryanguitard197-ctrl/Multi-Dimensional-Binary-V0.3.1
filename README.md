<p align="center">
  <img src="https://img.shields.io/badge/version-0.3.1-6366f1?style=for-the-badge" alt="Version">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=for-the-badge&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/target-WebAssembly-654ff0?style=for-the-badge&logo=webassembly" alt="WASM">
  <img src="https://img.shields.io/badge/license-MIT-green?style=for-the-badge" alt="License">
  <img src="https://img.shields.io/badge/runs_in-Any_Browser-06b6d4?style=for-the-badge" alt="Browser">
</p>

# MDB-OS — Multidimensional Binary Operating System

**A computational framework that replaces exponential memory scaling with sparse state representation. 64 positions. 2 states. 168 bytes.**

> *A distinct approach to representing and manipulating superposition-like states — running on commodity hardware.*

---

## The Problem

Every quantum simulator stores **2ⁿ complex amplitudes**. For 64 positions, that's **295 exabytes** — more memory than exists on Earth. This hard limit has been treated as an unavoidable constraint of the simulation approach.

## The Solution

MDB-OS introduces the **Sparse Register** — a `HashMap<u64, Complex>` that stores only the basis states that actually exist. The memory cost scales with **occupied states (k)**, not total state space (2ⁿ).

| | Quantum Hardware | Classical Simulator | **MDB-OS** |
|---|---|---|---|
| Memory at 64 positions | Physical hardware | 295 exabytes | **168 bytes** (sparse circuits) |
| Non-destructive readout | Impossible | Possible | **`peek()` — native** |
| State cloning | No-cloning theorem | Full copy (expensive) | **`fork()` — O(k) sparse** |
| Deterministic replay | Non-deterministic | Possible | **Seeded PRNG — native** |
| Runs on | $10M+ lab | Supercomputer | **Any browser** |
| Position limit | ~1,000 noisy | ~40 (memory wall) | **None** |

---

## Live Demo

**Try MDB-OS right now — no install, no download, no sign-up:**

🖥️ **[Launch MDB-OS Desktop →](https://preview-mdb-os-desktop-718d07ca.viktor.space)**

🌐 **[Landing Page](https://preview-mdb-os-landing-52741dcf.viktor.space)**

---

## Core Concepts

### 1. Sparse Register

The foundational data structure. Instead of allocating 2ⁿ amplitudes, the register stores only populated basis states in a hash map. A 64-position GHZ state has exactly 2 entries — `|000...0⟩` and `|111...1⟩` — consuming 168 bytes regardless of register width.

```
Traditional: Memory = O(2ⁿ) × 16 bytes
MDB-OS:      Memory = O(k) × 16 bytes   (k = populated states)
```

The memory advantage is strongest for circuits that maintain sparse state throughout execution — such as GHZ states, entanglement operations, and sparse oracle evaluations. Circuits that require full superposition across all basis states will populate the complete state space at that stage, scaling with 2ⁿ. Understanding which circuits stay sparse is key to getting the most from this architecture.

### 2. Three Operations Quantum Can't Have

| Operation | What It Does | Why Quantum Can't |
|---|---|---|
| **`peek()`** | Non-destructive state inspection | Quantum measurement collapses the wavefunction |
| **`fork()`** | Lossless state cloning in O(k) | No-cloning theorem forbids copying quantum states |
| **`replay()`** | Deterministic re-execution from seed | Quantum measurement is inherently probabilistic |

These aren't debug features — they're designed capabilities of the MDB framework that quantum hardware fundamentally cannot provide.

### 3. The SuperBit

Every binary pattern receives a geometric identity:

```
B = (σ, Ψ, W, A, G)

σ  — binary pattern (classical bits)
Ψ  — state space (possible interpretations)
W  — probability weight vector
A  — anchor positions (immutable bits)
G  — generation counter (lineage)
```

The **Dimensional Cascade** maps every SuperBit into ℝ∞ using Fibonacci-scaled coordinates that converge to the golden ratio **φ = 1.618034...**. Structurally similar binary strings cluster geometrically, turning certain search problems into spatial navigation rather than exhaustive enumeration.

### 4. Evolution

SuperBits evolve. Three modes:

- **Dimensional evolution** — flips a bit determined by the string's length parity
- **Learning evolution** — reweights probability distributions based on observed outcomes and rewards
- **Cascade evolution** — uses φ to drive a low-discrepancy traversal of all bit positions (the same golden-angle pattern found in sunflower spirals and leaf phyllotaxis)

All evolution supports non-destructive preview via `fork()` — the original state is always preserved.

### 5. Network Entanglement

SuperBits can be linked in a cascade-aware entanglement fabric. When one SuperBit evolves, the cascade change ripples through entanglement links to correlated partners — driven by the same φ that governs the cascade. Coupling strength and dimension are configurable per link.

### 6. Dimensional Scheduler

Processes are scheduled by geometric proximity in cascade coordinate space. Related workloads cluster spatially and get batched — a novel scheduling approach derived directly from the dimensional addressing system.

### 7. Cascade-Keyed Data Transformation

The `fold` module implements a deterministic, reversible data transformation keyed by cascade coordinates. The permutation and XOR mask are derived from the SuperBit's dimensional address, making the transformation unique to each data object's geometric identity. This is not compression — the output is the same size as the input — but it is a novel approach to data transformation where the key is intrinsic to the data's dimensional position rather than external.

---

## Desktop Applications

MDB-OS ships as a browser-based desktop environment with six built-in applications:

### 🖥️ Terminal
Full command-line interface to the MDB engine:
- `register <n>` — Create and inspect sparse registers
- `superbit <n>` / `cascade <n>` — Explore SuperBit geometry
- `circuit <type> <n>` — Build and run quantum circuits
- `grover <n>` — Run Grover's search algorithm
- `shor <n>` — Run Shor's factoring algorithm

### 🧪 Experiment Lab
- **Grover's Search** — Quadratic speedup on sparse oracle circuits
- **Deutsch-Jozsa** — Determine function properties in one evaluation
- **Bernstein-Vazirah** — Discover hidden bit strings
- **Quantum Phase Estimation** — Estimate eigenvalues
- **Variational Optimization** — Hybrid classical-quantum optimization
- **MDB Capabilities Demo** — Full showcase of peek/fork/replay

### ⚡ Circuit Designer
Visual quantum circuit builder with full gate palette: H, X, Y, Z, S, T, CNOT, CZ, SWAP, Toffoli, Fredkin, QFT. Preset circuits: Bell state, GHZ state, QFT.

### 📁 File Manager
Persistent file system (IndexedDB-backed) for saving experiments, circuits, and results.

### 🌀 Playground
Interactive exploration of Dimensional Cascade visualization, Grover's Search step-by-step, and Deutsch-Jozsa oracle testing.

### ⚙️ Settings
Customize theme, register defaults, and display preferences.

---

## Technical Stack

| Component | Technology |
|---|---|
| Core engine | Rust |
| Browser target | WebAssembly (322 KB) |
| UI | Single-file HTML + CSS + JS |
| Storage | IndexedDB (persistent) |
| Dependencies | Zero runtime dependencies |

The entire application is a single HTML file (~144 KB) plus the WASM binary (~322 KB). No build step needed for deployment — just serve the files.

---

## Gate Set

| Gate | Type | Description |
|---|---|---|
| H | Single | Hadamard — creates superposition |
| X, Y, Z | Single | Pauli gates |
| S, T | Single | Phase gates |
| Rx, Ry, Rz | Single | Rotation gates (parameterized) |
| CNOT | Two-qubit | Controlled NOT |
| CZ | Two-qubit | Controlled Z |
| SWAP | Two-qubit | State swap |
| Toffoli | Three-qubit | Controlled-controlled NOT |
| Fredkin | Three-qubit | Controlled SWAP |
| QFT | Multi-qubit | Quantum Fourier Transform |

---

## Algorithms

### Grover's Search
Finds a marked item in an unstructured database with **O(√N)** oracle queries instead of classical O(N). The quadratic speedup is realized for circuits where the oracle and diffusion steps maintain sparse state. Circuits requiring full superposition across all basis states will populate the complete state space at that stage.

### Deutsch-Jozsa
Determines whether a function is constant or balanced in a **single evaluation** — exponential speedup over classical.

### Bernstein-Vazirah
Discovers a hidden bit string in **one query** instead of n queries classically.

### Shor's Algorithm
Integer factoring via quantum period-finding on the sparse register. The current implementation uses quantum period-finding for N ≤ 20 bits and classical period-finding for larger N. Full quantum period-finding for large N is a development target for future versions.

### Variational Optimization
Hybrid approach using parameterized circuits with classical optimization of rotation angles. Configurable fitness functions, mutation rates, and retention percentages.

---

## Building from Source

### Prerequisites
- [Rust](https://rustup.rs/) (1.70+)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

### Build
```bash
git clone https://github.com/ryanguitard197-ctrl/Multi-Dimensional-Binary-V0.3.1.git
cd Multi-Dimensional-Binary-V0.3.1
cd wasm
wasm-pack build --target web --out-dir ../docs/pkg
cd ../docs
python3 -m http.server 8080
# Open http://localhost:8080/desktop.html
```

### No-Build Option
The `docs/` folder contains pre-built files. Open `docs/desktop.html` in any modern browser.

---

## Key Numbers

| Metric | Value |
|---|---|
| 64-position GHZ state memory | **168 bytes** |
| Same state in dense register | **295 exabytes** |
| WASM binary size | **322 KB** |
| Total app size | **~466 KB** |
| Runtime dependencies | **0** |
| Gate count | **12 universal gates** |
| Algorithm count | **6 verified algorithms** |
| Position limit | **None** |

---

## Author

**Ryan Guitard**

MDB-OS is the result of original research into sparse state representation as an alternative computational primitive. The framework is not a quantum simulator — it is a distinct approach to representing and manipulating superposition-like states using classical data structures, with novel OS-level primitives built from the ground up on a Fibonacci-recursive dimensional cascade addressing system.

---

## License

[MIT](LICENSE)

---

<p align="center">
  <strong>64 positions. 2 states. 168 bytes.</strong><br>
  <em>The foundation is built. The numbers are verified.</em>
</p>

