<p align="center">
  <img src="https://img.shields.io/badge/version-0.3.1-6366f1?style=for-the-badge" alt="Version">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=for-the-badge&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/target-WebAssembly-654ff0?style=for-the-badge&logo=webassembly" alt="WASM">
  <img src="https://img.shields.io/badge/license-MIT-green?style=for-the-badge" alt="License">
  <img src="https://img.shields.io/badge/runs_in-Any_Browser-06b6d4?style=for-the-badge" alt="Browser">
</p>

# MDB-OS — Multidimensional Binary Operating System

**A computational framework that replaces exponential memory scaling with sparse state representation. 64 positions. 2 states. 168 bytes.**

> *What quantum computing promised but couldn't deliver — running on commodity hardware.*

---

## The Problem

Every quantum simulator stores **2ⁿ complex amplitudes**. For 64 positions, that's **295 exabytes** — more memory than exists on Earth. This hard limit has been treated as an unavoidable law of physics.

## The Solution

MDB-OS introduces the **Sparse Register** — a `HashMap<u64, Complex>` that stores only the basis states that actually exist. The memory cost scales with **occupied states (k)**, not total state space (2ⁿ).

| | Quantum Hardware | Classical Simulator | **MDB-OS** |
|---|---|---|---|
| Memory at 64 positions | Physical hardware | 295 exabytes | **168 bytes** |
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

### 2. Three Operations Quantum Can't Have

| Operation | What It Does | Why Quantum Can't |
|---|---|---|
| **`peek()`** | Non-destructive state inspection | Quantum measurement collapses the wavefunction |
| **`fork()`** | Lossless state cloning in O(k) | No-cloning theorem forbids copying quantum states |
| **`replay()`** | Deterministic re-execution from seed | Quantum measurement is inherently probabilistic |

These aren't debug features — they're designed capabilities of the MDB framework.

### 3. The SuperBit

Every binary pattern receives a geometric identity:

```
B = (σ, Ψ, W, A, G)

σ  — binary pattern (classical bits)
Ψ  — phase component (golden ratio modulation)
W  — Hamming weight
A  — bit-length (amplitude)
G  — geometric coordinate (Fibonacci cascade sum)
```

The **Dimensional Cascade** maps every SuperBit into ℝ∞ using Fibonacci-scaled coordinates that converge to the golden ratio **φ = 1.618034...**. This turns search problems into geometric problems — navigating a space instead of enumerating it.

---

## Desktop Applications

MDB-OS ships as a browser-based desktop environment with six built-in applications:

### 🖥️ Terminal
Full command-line interface to the MDB engine. Commands include:
- `register <n>` — Create and inspect sparse registers
- `superbit <n>` / `cascade <n>` — Explore SuperBit geometry
- `circuit <type> <n>` — Build and run quantum circuits
- `grover <n>` — Run Grover's search algorithm
- `shor <n>` — Run Shor's factoring algorithm

### 🧪 Experiment Lab
Run comparative experiments:
- **Grover's Search** — Quadratic speedup on the sparse register
- **Deutsch-Jozsa** — Determine function properties in one evaluation
- **Bernstein-Vazirani** — Discover hidden bit strings
- **Quantum Phase Estimation** — Estimate eigenvalues
- **Variational Optimization** — Hybrid classical-quantum optimization
- **MDB Capabilities Demo** — Full showcase of peek/fork/replay

### ⚡ Circuit Designer
Visual quantum circuit builder with:
- Full gate palette: H, X, Y, Z, S, T, CNOT, CZ, SWAP, Toffoli, Fredkin, QFT
- Preset circuits: Bell state, GHZ state, QFT
- Real-time execution on the sparse register

### 📁 File Manager
Persistent file system (IndexedDB-backed) for saving experiments, circuits, and results.

### 🌀 Playground
Interactive exploration of:
- Dimensional Cascade visualization
- Grover's Search with step-by-step execution
- Deutsch-Jozsa oracle testing

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

The sparse register implements a complete universal gate set:

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

All gates operate directly on the sparse representation — only touching populated states.

---

## Algorithms

### Grover's Search
Finds a marked item in an unstructured database with **O(√N)** oracle queries instead of the classical O(N). Verified on the sparse register with correct quadratic speedup.

### Deutsch-Jozsa
Determines whether a function is constant or balanced in a **single evaluation** — exponential speedup over classical.

### Bernstein-Vazirani
Discovers a hidden bit string in **one query** instead of n queries classically.

### Shor's Algorithm
Integer factoring via quantum period-finding on the sparse register.

### Variational Optimization
Hybrid approach using parameterized circuits with classical optimization of rotation angles. Includes configurable fitness functions, mutation rates, and retention percentages.

---

## Project Structure

```
Multi-Dimensional-Binary-V0.3.1/
├── core/                      # Rust source code
│   ├── src/
│   │   ├── lib.rs             # Core MDB engine
│   │   ├── register.rs        # Sparse quantum register
│   │   ├── gates.rs           # Gate implementations
│   │   ├── circuit.rs         # Circuit builder
│   │   ├── algorithms.rs      # Grover, Deutsch-Jozsa, etc.
│   │   ├── superbit.rs        # SuperBit B = (σ, Ψ, W, A, G)
│   │   └── cascade.rs         # Dimensional cascade
│   └── Cargo.toml
├── wasm/                      # WebAssembly bindings
│   ├── src/lib.rs             # wasm-bindgen interface
│   └── Cargo.toml
├── docs/                      # Deployable web application
│   ├── index.html             # Landing page
│   ├── desktop.html           # MDB-OS Desktop
│   └── pkg/                   # Compiled WASM binary
│       ├── mdb_wasm.js
│       ├── mdb_wasm.d.ts
│       └── mdb_wasm_bg.wasm
├── README.md
└── LICENSE
```

---

## Building from Source

### Prerequisites
- [Rust](https://rustup.rs/) (1.70+)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

### Build
```bash
# Clone the repository
git clone https://github.com/ryanguitard197-ctrl/Multi-Dimensional-Binary-V0.3.1.git
cd Multi-Dimensional-Binary-V0.3.1

# Build the WASM binary
cd wasm
wasm-pack build --target web --out-dir ../docs/pkg

# Serve locally
cd ../docs
python3 -m http.server 8080
# Open http://localhost:8080/desktop.html
```

### No-Build Option
The `docs/` folder contains pre-built files. Just open `docs/desktop.html` in any modern browser — or serve the `docs/` directory with any static file server.

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

MDB-OS is the result of original research into sparse state representation as an alternative computational primitive. The framework is not a quantum simulator — it is a distinct approach to representing and manipulating superposition-like states using classical data structures.

---

## License

[MIT](LICENSE)

---

<p align="center">
  <strong>64 positions. 2 states. 168 bytes.</strong><br>
  <em>The foundation is built. The numbers are verified.</em>
</p>
