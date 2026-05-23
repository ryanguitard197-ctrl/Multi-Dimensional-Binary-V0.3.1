//! # MDB Process Model
//!
//! SuperBit-based process management for MDB-OS.
//!
//! ## Concept
//!
//! In traditional OS design, a process is a flat blob of memory plus some
//! kernel metadata (PID, state, registers). In MDB-OS, every process is a
//! *dimensional entity* — it has a SuperBit state vector, a coordinate address,
//! and it can be *folded* (hibernated into dimensional storage) and *unfolded*
//! (resumed) losslessly.
//!
//! ## Features
//!
//! - **Dimensional addressing**: Every process has a unique `DimensionalAddress`
//!   in D3/D4/D5 coordinate space, computed from its identity and state.
//!
//! - **Process folding**: A running process can be folded into MDB dimensional
//!   form — its entire state (memory, registers, file descriptors) is captured,
//!   geometrically folded, and stored in MDBFS. The process disappears from
//!   the scheduler but persists in dimensional space.
//!
//! - **Process unfolding**: A folded process can be unfolded back into a running
//!   state. All state is restored exactly. This is like hibernation but per-process
//!   and backed by MDB's dimensional guarantees.
//!
//! - **Evolution**: Processes accumulate experience. The MDB evolution engine
//!   tracks access patterns, resource usage, and execution history. Over time,
//!   the system learns optimal scheduling and resource allocation.
//!
//! - **Entanglement**: Related processes (e.g., a web server and its database)
//!   can be linked via MDB network connections. When one is folded/unfolded,
//!   the system knows about its dependencies.

pub mod manager;
pub mod process;
pub mod scheduler;

pub use manager::ProcessManager;
pub use process::{MdbProcess, ProcessState};
pub use scheduler::DimensionalScheduler;
