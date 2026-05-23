//! # MDB Process
//!
//! Represents a process in the MDB dimensional model.

use mdb_core::coordinates::DimensionalAddress;
use mdb_core::fold::{fold, FoldedData};
use mdb_core::superbit::SuperBit;
use mdb_core::unfold::unfold;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Unique process identifier in MDB-OS.
pub type MdbPid = u64;

/// Process lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    /// Process is being created.
    Creating,
    /// Process is running (scheduled on CPU).
    Running,
    /// Process is ready to run (waiting for CPU).
    Ready,
    /// Process is blocked (waiting for I/O, lock, etc.).
    Blocked,
    /// Process is folded (hibernated in MDB dimensional storage).
    Folded,
    /// Process is being unfolded (resuming from dimensional storage).
    Unfolding,
    /// Process has terminated.
    Terminated,
    /// Process terminated abnormally.
    Failed,
}

/// Resource usage tracking for a process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Total CPU time consumed.
    pub cpu_time: Duration,
    /// Peak memory usage in bytes.
    pub peak_memory: u64,
    /// Current memory usage in bytes.
    pub current_memory: u64,
    /// Total bytes read from storage.
    pub io_read_bytes: u64,
    /// Total bytes written to storage.
    pub io_write_bytes: u64,
    /// Number of fold/unfold cycles.
    pub fold_cycles: u32,
    /// Total time spent in folded state.
    pub time_folded: Duration,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_time: Duration::ZERO,
            peak_memory: 0,
            current_memory: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            fold_cycles: 0,
            time_folded: Duration::ZERO,
        }
    }
}

/// An MDB process — a dimensional entity with SuperBit state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdbProcess {
    /// MDB process ID.
    pub pid: MdbPid,
    /// Underlying OS PID (when running).
    pub os_pid: Option<u32>,
    /// Process name.
    pub name: String,
    /// Command line.
    pub command: Vec<String>,
    /// Current lifecycle state.
    pub state: ProcessState,

    /// Dimensional address — where this process lives in MDB coordinate space.
    pub address: DimensionalAddress,
    /// SuperBit state vector — the process's dimensional representation.
    pub superbit: SuperBit,

    /// Resource usage tracking.
    pub usage: ResourceUsage,
    /// When the process was created.
    pub created_at: SystemTime,
    /// When the process last changed state.
    pub state_changed_at: SystemTime,

    /// If folded, the serialized state data.
    pub folded_state: Option<Vec<u8>>,
    /// Connected processes (entanglement).
    pub entangled_pids: Vec<MdbPid>,
    /// Custom metadata.
    pub metadata: HashMap<String, String>,

    /// Evolution state — tracks patterns for learning.
    pub evolution: ProcessEvolution,
}

/// Evolution tracking for a process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvolution {
    /// How many times this process has been observed.
    pub observation_count: u64,
    /// Average CPU time per run.
    pub avg_cpu_time: Duration,
    /// Average memory usage.
    pub avg_memory: u64,
    /// Predicted next run duration.
    pub predicted_runtime: Duration,
    /// Priority weight (learned from history).
    pub learned_priority: f64,
    /// Dimensional affinity scores — which D3/D4/D5 regions this
    /// process tends to interact with.
    pub dimensional_affinity: [f64; 5],
}

impl Default for ProcessEvolution {
    fn default() -> Self {
        Self {
            observation_count: 0,
            avg_cpu_time: Duration::ZERO,
            avg_memory: 0,
            predicted_runtime: Duration::ZERO,
            learned_priority: 1.0,
            dimensional_affinity: [0.0; 5],
        }
    }
}

impl MdbProcess {
    /// Create a new process.
    pub fn new(pid: MdbPid, name: String, command: Vec<String>) -> Self {
        let now = SystemTime::now();

        // Create a SuperBit that represents this process's identity
        let identity_bytes = format!("{}:{}:{:?}", pid, name, command);
        let superbit = SuperBit::from_bytes(identity_bytes.as_bytes());
        let address = DimensionalAddress::from_bytes(identity_bytes.as_bytes());

        Self {
            pid,
            os_pid: None,
            name,
            command,
            state: ProcessState::Creating,
            address,
            superbit,
            usage: ResourceUsage::default(),
            created_at: now,
            state_changed_at: now,
            folded_state: None,
            entangled_pids: Vec::new(),
            metadata: HashMap::new(),
            evolution: ProcessEvolution::default(),
        }
    }

    /// Transition to a new state.
    pub fn transition(&mut self, new_state: ProcessState) {
        log::debug!(
            "Process {} ({}) {:?} → {:?}",
            self.pid,
            self.name,
            self.state,
            new_state
        );
        self.state = new_state;
        self.state_changed_at = SystemTime::now();
    }

    /// Fold (hibernate) this process.
    ///
    /// Captures the process state, folds it into MDB dimensional form,
    /// and stores the result. The OS-level process can then be suspended
    /// or terminated — it will be fully restored on unfold.
    pub fn fold_process(&mut self) -> Result<FoldedData, String> {
        if self.state == ProcessState::Folded {
            return Err("Process is already folded".to_string());
        }

        // Serialize the complete process state
        let state_bytes = serde_json::to_vec(self)
            .map_err(|e| format!("Failed to serialize process state: {}", e))?;

        // Fold through MDB dimensional engine
        let folded = fold(&state_bytes);

        // Update state
        self.folded_state = Some(mdb_core::fold::encode_folded(&folded));
        self.transition(ProcessState::Folded);
        self.usage.fold_cycles += 1;

        log::info!(
            "Process {} folded: {} bytes → {} bytes folded at address D3={} D4={:.6}",
            self.pid,
            state_bytes.len(),
            folded.payload.len(),
            self.address.n,
            self.address.d4_spacetime,
        );

        Ok(folded)
    }

    /// Unfold (resume) a folded process.
    ///
    /// Restores the process from its MDB dimensional form back to a
    /// runnable state. All state is recovered losslessly.
    pub fn unfold_process(folded_bytes: &[u8]) -> Result<Self, String> {
        // Decode and unfold
        let folded = mdb_core::fold::decode_folded(folded_bytes)
            .map_err(|e| format!("Failed to decode folded data: {}", e))?;
        let state_bytes = unfold(&folded)
            .map_err(|e| format!("Failed to unfold data: {}", e))?;

        // Deserialize the process
        let mut process: Self = serde_json::from_slice(&state_bytes)
            .map_err(|e| format!("Failed to deserialize process state: {}", e))?;

        process.transition(ProcessState::Ready);
        process.folded_state = None;

        log::info!(
            "Process {} unfolded from dimensional storage",
            process.pid,
        );

        Ok(process)
    }

    /// Update evolution metrics based on current run.
    pub fn evolve(&mut self) {
        let evo = &mut self.evolution;
        evo.observation_count += 1;

        let n = evo.observation_count as f64;

        // Running average of CPU time
        let cpu_micros = self.usage.cpu_time.as_micros() as f64;
        let avg_micros = evo.avg_cpu_time.as_micros() as f64;
        let new_avg = avg_micros + (cpu_micros - avg_micros) / n;
        evo.avg_cpu_time = Duration::from_micros(new_avg as u64);

        // Running average of memory
        let new_mem_avg = evo.avg_memory as f64
            + (self.usage.current_memory as f64 - evo.avg_memory as f64) / n;
        evo.avg_memory = new_mem_avg as u64;

        // Predict next runtime (exponential moving average)
        let alpha = 0.3;
        let pred = evo.predicted_runtime.as_micros() as f64;
        let new_pred = alpha * cpu_micros + (1.0 - alpha) * pred;
        evo.predicted_runtime = Duration::from_micros(new_pred as u64);

        // Update dimensional affinity based on address
        evo.dimensional_affinity[2] =
            evo.dimensional_affinity[2] * 0.9 + (self.address.n as f64 / self.address.n.max(1) as f64).min(1.0) * 0.1;
        evo.dimensional_affinity[3] =
            evo.dimensional_affinity[3] * 0.9 + self.address.d4_spacetime * 0.1;
        evo.dimensional_affinity[4] =
            evo.dimensional_affinity[4] * 0.9 + self.address.d5_momentum as f64 * 0.1;
    }

    /// Entangle this process with another (create a dependency link).
    pub fn entangle(&mut self, other_pid: MdbPid) {
        if !self.entangled_pids.contains(&other_pid) {
            self.entangled_pids.push(other_pid);
        }
    }

    /// Check if this process is entangled with another.
    pub fn is_entangled_with(&self, other_pid: MdbPid) -> bool {
        self.entangled_pids.contains(&other_pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_process() {
        let proc = MdbProcess::new(1, "test".into(), vec!["echo".into(), "hello".into()]);
        assert_eq!(proc.pid, 1);
        assert_eq!(proc.state, ProcessState::Creating);
        assert_eq!(proc.name, "test");
    }

    #[test]
    fn test_fold_unfold_process() {
        let mut proc = MdbProcess::new(42, "myapp".into(), vec!["/usr/bin/myapp".into()]);
        proc.transition(ProcessState::Running);
        proc.usage.cpu_time = Duration::from_millis(150);
        proc.usage.current_memory = 1024 * 1024;
        proc.metadata.insert("env".into(), "production".into());

        // Fold
        let _folded = proc.fold_process().unwrap();
        assert_eq!(proc.state, ProcessState::Folded);
        assert!(proc.folded_state.is_some());

        // Unfold
        let restored = MdbProcess::unfold_process(proc.folded_state.as_ref().unwrap()).unwrap();
        assert_eq!(restored.pid, 42);
        assert_eq!(restored.name, "myapp");
        assert_eq!(restored.state, ProcessState::Ready);
        assert_eq!(restored.usage.cpu_time, Duration::from_millis(150));
        assert_eq!(restored.metadata.get("env").unwrap(), "production");
    }

    #[test]
    fn test_entanglement() {
        let mut proc1 = MdbProcess::new(1, "server".into(), vec![]);
        let proc2 = MdbProcess::new(2, "database".into(), vec![]);

        proc1.entangle(proc2.pid);
        assert!(proc1.is_entangled_with(2));
        assert!(!proc1.is_entangled_with(3));
    }

    #[test]
    fn test_evolution() {
        let mut proc = MdbProcess::new(1, "test".into(), vec![]);
        proc.usage.cpu_time = Duration::from_millis(100);
        proc.usage.current_memory = 5000;

        proc.evolve();
        assert_eq!(proc.evolution.observation_count, 1);
        assert!(proc.evolution.avg_cpu_time.as_millis() > 0);

        proc.usage.cpu_time = Duration::from_millis(200);
        proc.evolve();
        assert_eq!(proc.evolution.observation_count, 2);
    }
}
