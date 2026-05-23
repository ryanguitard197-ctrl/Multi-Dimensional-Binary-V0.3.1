//! # Process Manager
//!
//! Manages the lifecycle of all MDB processes: creation, scheduling,
//! folding/unfolding, and termination.

use crate::process::{MdbPid, MdbProcess, ProcessState};
use crate::scheduler::DimensionalScheduler;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The MDB Process Manager.
///
/// Owns all processes and coordinates with the scheduler and MDBFS
/// for process folding/unfolding.
pub struct ProcessManager {
    /// All known processes (running + folded).
    processes: HashMap<MdbPid, MdbProcess>,
    /// The dimensional scheduler.
    scheduler: DimensionalScheduler,
    /// Next PID to assign.
    next_pid: MdbPid,
    /// Path to MDBFS store for folded process persistence.
    fold_store_path: PathBuf,
}

impl ProcessManager {
    /// Create a new process manager.
    pub fn new(fold_store_path: PathBuf) -> Self {
        Self {
            processes: HashMap::new(),
            scheduler: DimensionalScheduler::new(),
            next_pid: 1,
            fold_store_path,
        }
    }

    /// Spawn a new process.
    pub fn spawn(&mut self, name: String, command: Vec<String>) -> Result<MdbPid, String> {
        let pid = self.next_pid;
        self.next_pid += 1;

        let mut process = MdbProcess::new(pid, name.clone(), command.clone());

        // Transition to ready
        process.transition(ProcessState::Ready);

        // Add to scheduler
        self.scheduler.enqueue(&process);

        log::info!(
            "Spawned process {} (pid={}, address=n:{} D4:{:.4})",
            name,
            pid,
            process.address.n,
            process.address.d4_spacetime,
        );

        self.processes.insert(pid, process);
        Ok(pid)
    }

    /// Fold (hibernate) a process to dimensional storage.
    pub fn fold(&mut self, pid: MdbPid) -> Result<(), String> {
        let process = self
            .processes
            .get_mut(&pid)
            .ok_or_else(|| format!("No process with pid {}", pid))?;

        // Fold through MDB engine
        let folded = process.fold_process()?;

        // Persist folded state to MDBFS store
        let fold_dir = self.fold_store_path.join("folded_processes");
        std::fs::create_dir_all(&fold_dir)
            .map_err(|e| format!("Failed to create fold directory: {}", e))?;

        let fold_path = fold_dir.join(format!("{}.mdb", pid));
        let encoded = mdb_core::fold::encode_folded(&folded);
        std::fs::write(&fold_path, &encoded)
            .map_err(|e| format!("Failed to write folded process: {}", e))?;

        log::info!("Process {} folded to {}", pid, fold_path.display());
        Ok(())
    }

    /// Unfold (resume) a process from dimensional storage.
    pub fn unfold(&mut self, pid: MdbPid) -> Result<(), String> {
        let fold_path = self
            .fold_store_path
            .join("folded_processes")
            .join(format!("{}.mdb", pid));

        let data = std::fs::read(&fold_path)
            .map_err(|e| format!("Failed to read folded process {}: {}", pid, e))?;

        let process = MdbProcess::unfold_process(&data)?;

        // Re-add to scheduler
        self.scheduler.enqueue(&process);
        self.processes.insert(pid, process);

        // Clean up fold file
        let _ = std::fs::remove_file(&fold_path);

        log::info!("Process {} unfolded and ready", pid);
        Ok(())
    }

    /// Get the next process to run from the scheduler.
    pub fn schedule_next(&mut self) -> Option<MdbPid> {
        self.scheduler.dequeue()
    }

    /// Terminate a process.
    pub fn terminate(&mut self, pid: MdbPid) -> Result<(), String> {
        let process = self
            .processes
            .get_mut(&pid)
            .ok_or_else(|| format!("No process with pid {}", pid))?;

        // Evolve before terminating (learn from this run)
        process.evolve();
        process.transition(ProcessState::Terminated);

        log::info!("Process {} terminated", pid);
        Ok(())
    }

    /// Entangle two processes (create a dependency link).
    pub fn entangle(&mut self, pid_a: MdbPid, pid_b: MdbPid) -> Result<(), String> {
        // Verify both exist
        if !self.processes.contains_key(&pid_a) {
            return Err(format!("No process with pid {}", pid_a));
        }
        if !self.processes.contains_key(&pid_b) {
            return Err(format!("No process with pid {}", pid_b));
        }

        // Link both directions
        self.processes.get_mut(&pid_a).unwrap().entangle(pid_b);
        self.processes.get_mut(&pid_b).unwrap().entangle(pid_a);

        log::info!("Entangled processes {} ↔ {}", pid_a, pid_b);
        Ok(())
    }

    /// Get a process by PID.
    pub fn get(&self, pid: MdbPid) -> Option<&MdbProcess> {
        self.processes.get(&pid)
    }

    /// List all processes.
    pub fn list(&self) -> Vec<&MdbProcess> {
        self.processes.values().collect()
    }

    /// List folded processes (persisted on disk).
    pub fn list_folded(&self) -> Vec<MdbPid> {
        let fold_dir = self.fold_store_path.join("folded_processes");
        let mut pids = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&fold_dir) {
            for entry in entries.flatten() {
                if let Some(stem) = entry.path().file_stem() {
                    if let Ok(pid) = stem.to_string_lossy().parse::<MdbPid>() {
                        pids.push(pid);
                    }
                }
            }
        }
        pids
    }

    /// Get system-wide stats.
    pub fn stats(&self) -> ManagerStats {
        let mut running = 0;
        let mut ready = 0;
        let mut folded = 0;
        let mut blocked = 0;
        let mut total_memory = 0u64;

        for proc in self.processes.values() {
            match proc.state {
                ProcessState::Running => running += 1,
                ProcessState::Ready => ready += 1,
                ProcessState::Folded => folded += 1,
                ProcessState::Blocked => blocked += 1,
                _ => {}
            }
            total_memory += proc.usage.current_memory;
        }

        ManagerStats {
            total_processes: self.processes.len(),
            running,
            ready,
            folded,
            blocked,
            total_memory,
            scheduler_tick: self.scheduler.current_tick(),
            scheduler_queue_len: self.scheduler.queue_len(),
        }
    }
}

/// System-wide process statistics.
#[derive(Debug)]
pub struct ManagerStats {
    pub total_processes: usize,
    pub running: usize,
    pub ready: usize,
    pub folded: usize,
    pub blocked: usize,
    pub total_memory: u64,
    pub scheduler_tick: u64,
    pub scheduler_queue_len: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_spawn_and_list() {
        let mut mgr = ProcessManager::new(PathBuf::from("/tmp/mdb-test-processes"));
        let pid = mgr.spawn("test".into(), vec!["echo".into()]).unwrap();
        assert_eq!(pid, 1);
        assert_eq!(mgr.list().len(), 1);
    }

    #[test]
    fn test_fold_unfold_cycle() {
        let dir = PathBuf::from("/tmp/mdb-process-test-fold");
        let _ = std::fs::create_dir_all(&dir);

        let mut mgr = ProcessManager::new(dir.clone());
        let pid = mgr.spawn("foldme".into(), vec!["sleep".into(), "100".into()]).unwrap();

        // Fold
        mgr.fold(pid).unwrap();
        assert_eq!(mgr.get(pid).unwrap().state, ProcessState::Folded);

        // Unfold
        mgr.unfold(pid).unwrap();
        assert_eq!(mgr.get(pid).unwrap().state, ProcessState::Ready);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_entanglement() {
        let mut mgr = ProcessManager::new(PathBuf::from("/tmp/mdb-test-ent"));
        let p1 = mgr.spawn("server".into(), vec![]).unwrap();
        let p2 = mgr.spawn("db".into(), vec![]).unwrap();

        mgr.entangle(p1, p2).unwrap();
        assert!(mgr.get(p1).unwrap().is_entangled_with(p2));
        assert!(mgr.get(p2).unwrap().is_entangled_with(p1));
    }

    #[test]
    fn test_stats() {
        let mut mgr = ProcessManager::new(PathBuf::from("/tmp/mdb-test-stats"));
        mgr.spawn("a".into(), vec![]).unwrap();
        mgr.spawn("b".into(), vec![]).unwrap();

        let stats = mgr.stats();
        assert_eq!(stats.total_processes, 2);
        assert_eq!(stats.ready, 2);
    }
}
