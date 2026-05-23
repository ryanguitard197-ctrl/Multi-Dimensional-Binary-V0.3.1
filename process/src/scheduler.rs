//! # Dimensional Scheduler
//!
//! Schedules processes using MDB dimensional coordinates.
//!
//! Unlike a traditional round-robin or CFS scheduler, the MDB scheduler
//! uses dimensional locality — processes that are "near" each other in
//! coordinate space (related workloads) are scheduled together for
//! cache efficiency and reduced context-switch overhead.

use crate::process::{MdbPid, MdbProcess, ProcessState};
use mdb_core::coordinates::DimensionalAddress;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// A process ready to be scheduled, with a computed priority.
#[derive(Debug)]
struct ScheduleEntry {
    pid: MdbPid,
    priority: f64,
    address: DimensionalAddress,
}

impl PartialEq for ScheduleEntry {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}

impl Eq for ScheduleEntry {}

impl PartialOrd for ScheduleEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduleEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        self.priority
            .partial_cmp(&other.priority)
            .unwrap_or(Ordering::Equal)
    }
}

/// The MDB Dimensional Scheduler.
///
/// ## Scheduling algorithm
///
/// 1. Compute priority for each ready process based on:
///    - Base priority (from evolution learning)
///    - Dimensional locality (prefer processes near the current execution region)
///    - Starvation prevention (boost processes that haven't run recently)
///    - I/O readiness (boost processes with pending I/O completions)
///
/// 2. Group processes by dimensional quadrant (D3 regions) for batch scheduling.
///
/// 3. Within each batch, order by computed priority.
///
/// The scheduler periodically consults the evolution engine to update
/// learned priorities based on observed behaviour.
pub struct DimensionalScheduler {
    /// Ready queue (priority heap).
    ready_queue: BinaryHeap<ScheduleEntry>,
    /// Current dimensional focus region — the scheduler prefers processes
    /// whose addresses are near this point.
    focus_address: DimensionalAddress,
    /// Weight for dimensional locality in priority computation (0.0–1.0).
    locality_weight: f64,
    /// Weight for learned priority (0.0–1.0).
    evolution_weight: f64,
    /// Starvation threshold in scheduler ticks.
    starvation_threshold: u64,
    /// Current tick counter.
    tick: u64,
}

impl DimensionalScheduler {
    pub fn new() -> Self {
        Self {
            ready_queue: BinaryHeap::new(),
            focus_address: DimensionalAddress {
                n: 0,
                d4_spacetime: 0.0,
                d5_momentum: 0,
            },
            locality_weight: 0.3,
            evolution_weight: 0.4,
            starvation_threshold: 100,
            tick: 0,
        }
    }

    /// Enqueue a process for scheduling.
    pub fn enqueue(&mut self, process: &MdbProcess) {
        if process.state != ProcessState::Ready && process.state != ProcessState::Running {
            return;
        }

        let priority = self.compute_priority(process);

        self.ready_queue.push(ScheduleEntry {
            pid: process.pid,
            priority,
            address: process.address.clone(),
        });
    }

    /// Dequeue the highest-priority process.
    pub fn dequeue(&mut self) -> Option<MdbPid> {
        self.tick += 1;

        if let Some(entry) = self.ready_queue.pop() {
            // Shift focus toward the dequeued process's address
            self.update_focus(&entry.address);
            Some(entry.pid)
        } else {
            None
        }
    }

    /// Compute scheduling priority for a process.
    fn compute_priority(&self, process: &MdbProcess) -> f64 {
        // 1. Base learned priority from evolution
        let learned = process.evolution.learned_priority;

        // 2. Dimensional locality bonus
        let locality = self.dimensional_distance(&process.address);
        let locality_bonus = 1.0 / (1.0 + locality);

        // 3. Starvation prevention
        let time_waiting = process
            .state_changed_at
            .elapsed()
            .unwrap_or_default()
            .as_millis() as f64;
        let starvation_bonus = (time_waiting / 1000.0).min(2.0);

        // Combined priority
        let priority = (self.evolution_weight * learned)
            + (self.locality_weight * locality_bonus)
            + (0.2 * starvation_bonus)
            + 0.1; // base

        priority
    }

    /// Compute distance between focus and a process address in dimensional space.
    fn dimensional_distance(&self, addr: &DimensionalAddress) -> f64 {
        let d3_diff = (self.focus_address.n as f64 - addr.n as f64).abs() / (addr.n.max(self.focus_address.n).max(1) as f64);
        let d4_diff = (self.focus_address.d4_spacetime - addr.d4_spacetime).abs();
        let d5_diff = (self.focus_address.d5_momentum as f64 - addr.d5_momentum as f64).abs();

        (d3_diff * d3_diff + d4_diff * d4_diff + d5_diff * d5_diff).sqrt()
    }

    /// Smoothly move the focus address toward a target.
    fn update_focus(&mut self, target: &DimensionalAddress) {
        let alpha = 0.2; // Smoothing factor
        self.focus_address.n = ((1.0 - alpha) * self.focus_address.n as f64
            + alpha * target.n as f64) as u64;
        self.focus_address.d4_spacetime =
            (1.0 - alpha) * self.focus_address.d4_spacetime + alpha * target.d4_spacetime;
        self.focus_address.d5_momentum =
            ((1.0 - alpha) * self.focus_address.d5_momentum as f64 + alpha * target.d5_momentum as f64) as u64;
    }

    /// Get queue length.
    pub fn queue_len(&self) -> usize {
        self.ready_queue.len()
    }

    /// Get current tick.
    pub fn current_tick(&self) -> u64 {
        self.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_basic() {
        let mut sched = DimensionalScheduler::new();

        let mut p1 = MdbProcess::new(1, "high".into(), vec![]);
        p1.transition(ProcessState::Ready);
        p1.evolution.learned_priority = 2.0;

        let mut p2 = MdbProcess::new(2, "low".into(), vec![]);
        p2.transition(ProcessState::Ready);
        p2.evolution.learned_priority = 0.5;

        sched.enqueue(&p1);
        sched.enqueue(&p2);

        // Higher priority dequeued first
        let first = sched.dequeue().unwrap();
        assert_eq!(first, 1);
    }

    #[test]
    fn test_scheduler_empty() {
        let mut sched = DimensionalScheduler::new();
        assert!(sched.dequeue().is_none());
    }

    #[test]
    fn test_scheduler_locality() {
        let mut sched = DimensionalScheduler::new();

        // Set focus near process 1's address
        sched.focus_address = DimensionalAddress {
            n: 100, d4_spacetime: 0.5, d5_momentum: 1,
        };
        sched.locality_weight = 0.8;
        sched.evolution_weight = 0.1;

        let mut p1 = MdbProcess::new(1, "near".into(), vec![]);
        p1.transition(ProcessState::Ready);
        p1.address = DimensionalAddress {
            n: 100, d4_spacetime: 0.5, d5_momentum: 1,
        };

        let mut p2 = MdbProcess::new(2, "far".into(), vec![]);
        p2.transition(ProcessState::Ready);
        p2.address = DimensionalAddress {
            n: 0, d4_spacetime: 0.0, d5_momentum: 0,
        };

        sched.enqueue(&p1);
        sched.enqueue(&p2);

        // Nearby process should be preferred
        let first = sched.dequeue().unwrap();
        assert_eq!(first, 1);
    }
}
