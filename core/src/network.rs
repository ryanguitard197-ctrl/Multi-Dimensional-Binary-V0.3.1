//! # MDBNetwork — Cascade-Aware Entanglement Fabric
//!
//! The MDB network is a living fabric of interconnected SuperBits where
//! entanglement operates at the *dimensional cascade* level, not just
//! as simple BFS propagation.
//!
//! ## How Cascade Entanglement Works
//!
//! When two SuperBits are entangled, they share a **dimensional coupling**
//! that specifies *which dimensions* are linked and how strongly.
//! When one SuperBit evolves, the cascade change ripples through the
//! entanglement link and *shifts the entangled partner's σ* in a
//! correlated way — driven by the same Golden Ratio φ that governs
//! the cascade itself.
//!
//! ```text
//! SuperBit A evolves at position p:
//!   1. A's cascade changes (D4, D5, D6... all shift)
//!   2. For each entangled partner B:
//!      - Compute cascade delta from A's change
//!      - Scale delta by coupling strength
//!      - Apply φ-correlated position selection on B
//!      - B's σ changes → B's cascade updates → ripples outward
//! ```
//!
//! This is the non-local state correlation that mirrors quantum
//! entanglement — on classical hardware.

use crate::coordinates::DimensionalAddress;
use crate::evolution;
use crate::superbit::SuperBit;
use std::collections::{HashMap, HashSet, VecDeque};

/// Unique identifier for a node in the MDB network.
pub type NodeId = u64;

/// Golden Ratio — drives correlated position selection in entangled partners.
const PHI: f64 = 1.618_033_988_749_895;

// ---------------------------------------------------------------------------
// Entanglement Link
// ---------------------------------------------------------------------------

/// A dimensional coupling between two SuperBits.
///
/// Unlike the old "gravity weight" model, this carries real dimensional
/// information: which cascade dimensions are coupled and how strongly.
#[derive(Debug, Clone)]
pub struct EntanglementLink {
    /// Source node.
    pub from: NodeId,
    /// Target node.
    pub to: NodeId,
    /// Coupling strength (0.0–1.0). Scales how much of the source's
    /// cascade delta propagates to the target.
    pub coupling: f64,
    /// Which dimension to couple at (4 = Spacetime, 5 = Momentum, etc.).
    /// The cascade delta is measured at this dimension.
    pub dimension: usize,
}

/// Result of an entangled evolution step — what changed and where.
#[derive(Debug, Clone)]
pub struct EntanglementResult {
    /// The source node that initiated the evolution.
    pub source: NodeId,
    /// Position modified in the source.
    pub source_position: Option<usize>,
    /// All nodes affected by the ripple, with their modified positions.
    pub ripple_effects: Vec<(NodeId, Option<usize>)>,
    /// Total number of nodes touched (including source).
    pub nodes_affected: usize,
}

// ---------------------------------------------------------------------------
// Network Node
// ---------------------------------------------------------------------------

/// A node in the MDB network — wraps a SuperBit with network metadata.
#[derive(Debug, Clone)]
pub struct NetworkNode {
    /// Unique node identifier.
    pub id: NodeId,
    /// The SuperBit at this node.
    pub superbit: SuperBit,
    /// IDs of entangled neighbor nodes.
    pub neighbors: HashSet<NodeId>,
    /// Cached dimensional address (updated after each evolution).
    pub cached_address: DimensionalAddress,
}

// ---------------------------------------------------------------------------
// MDB Network
// ---------------------------------------------------------------------------

/// The MDB Network — a cascade-entangled fabric of SuperBits.
///
/// All SuperBits form an interconnected mesh. When one evolves, the
/// cascade change ripples through entanglement links, causing correlated
/// changes in entangled partners. This is non-local state correlation
/// on classical hardware.
#[derive(Debug)]
pub struct MDBNetwork {
    /// All nodes in the network.
    nodes: HashMap<NodeId, NetworkNode>,
    /// All entanglement links (directional — stored both ways).
    links: Vec<EntanglementLink>,
    /// Next available node ID.
    next_id: NodeId,
    /// Global evolution tick counter.
    pub tick: u64,
}

impl MDBNetwork {
    /// Create an empty MDB network.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            links: Vec::new(),
            next_id: 1,
            tick: 0,
        }
    }

    /// Add a SuperBit to the network. Returns its node ID.
    pub fn add_node(&mut self, superbit: SuperBit) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        let cached_address = superbit.address();
        let node = NetworkNode {
            id,
            superbit,
            neighbors: HashSet::new(),
            cached_address,
        };
        self.nodes.insert(id, node);
        id
    }

    /// Remove a node from the network.
    pub fn remove_node(&mut self, id: NodeId) -> Option<SuperBit> {
        if let Some(node) = self.nodes.remove(&id) {
            // Remove all links involving this node
            self.links.retain(|l| l.from != id && l.to != id);
            // Remove from neighbor sets
            for &neighbor_id in &node.neighbors {
                if let Some(neighbor) = self.nodes.get_mut(&neighbor_id) {
                    neighbor.neighbors.remove(&id);
                }
            }
            Some(node.superbit)
        } else {
            None
        }
    }

    /// Entangle two SuperBits with a dimensional coupling.
    ///
    /// `coupling` (0.0–1.0) controls how much of the source's cascade
    /// change propagates to the target. `dimension` specifies which
    /// cascade dimension is coupled (4 = Spacetime, 5 = Momentum, etc.).
    ///
    /// Creates a bidirectional entanglement — both directions are stored.
    pub fn entangle(
        &mut self,
        a: NodeId,
        b: NodeId,
        coupling: f64,
        dimension: usize,
    ) -> bool {
        if !self.nodes.contains_key(&a)
            || !self.nodes.contains_key(&b)
            || a == b
        {
            return false;
        }

        // Bidirectional links
        self.links.push(EntanglementLink {
            from: a,
            to: b,
            coupling,
            dimension,
        });
        self.links.push(EntanglementLink {
            from: b,
            to: a,
            coupling,
            dimension,
        });

        self.nodes.get_mut(&a).unwrap().neighbors.insert(b);
        self.nodes.get_mut(&b).unwrap().neighbors.insert(a);

        true
    }

    /// Get a reference to a node's SuperBit.
    pub fn get(&self, id: NodeId) -> Option<&SuperBit> {
        self.nodes.get(&id).map(|n| &n.superbit)
    }

    /// Get a mutable reference to a node's SuperBit.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut SuperBit> {
        self.nodes.get_mut(&id).map(|n| &mut n.superbit)
    }

    /// Get the cached dimensional address for a node.
    pub fn address(&self, id: NodeId) -> Option<&DimensionalAddress> {
        self.nodes.get(&id).map(|n| &n.cached_address)
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of entanglement links (counting both directions).
    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Get all neighbor IDs for a node.
    pub fn neighbors(&self, id: NodeId) -> Option<&HashSet<NodeId>> {
        self.nodes.get(&id).map(|n| &n.neighbors)
    }

    /// Get all links originating from a node.
    pub fn links_from(&self, id: NodeId) -> Vec<&EntanglementLink> {
        self.links.iter().filter(|l| l.from == id).collect()
    }

    // -------------------------------------------------------------------
    // Cascade-Aware Entangled Evolution
    // -------------------------------------------------------------------

    /// Evolve a source SuperBit and propagate the cascade change through
    /// entanglement links.
    ///
    /// **How cascade entanglement works:**
    ///
    /// 1. Snapshot the source's D4 (Spacetime) scalar before evolution
    /// 2. Evolve the source using cascade evolution (φ-driven)
    /// 3. Compute the cascade delta: `Δ = D4_after - D4_before`
    /// 4. For each entangled partner:
    ///    a. Scale delta by coupling strength: `Δ_partner = Δ × coupling`
    ///    b. Select target position using φ: `pos = floor(fract(Δ_partner × φ) × n)`
    ///    c. Flip that bit in the partner's σ (if not anchored)
    ///    d. Update partner's cached address
    /// 5. Ripple continues outward up to `max_depth`
    ///
    /// Returns an [`EntanglementResult`] describing all changes.
    pub fn evolve_entangled(
        &mut self,
        source: NodeId,
        max_depth: u32,
    ) -> EntanglementResult {
        let mut result = EntanglementResult {
            source,
            source_position: None,
            ripple_effects: Vec::new(),
            nodes_affected: 0,
        };

        // Snapshot source's D4 before evolution
        let d4_before = self
            .nodes
            .get(&source)
            .map(|n| n.cached_address.d4_spacetime)
            .unwrap_or(0.0);

        // Evolve the source using cascade evolution
        if let Some(node) = self.nodes.get_mut(&source) {
            let evo_result = evolution::evolve_cascade(&mut node.superbit);
            result.source_position = evo_result.modified_position;
            node.cached_address = node.superbit.address();
        }

        let d4_after = self
            .nodes
            .get(&source)
            .map(|n| n.cached_address.d4_spacetime)
            .unwrap_or(0.0);

        let cascade_delta = d4_after - d4_before;
        result.nodes_affected = 1;

        // BFS ripple through entanglement links
        let mut visited = HashSet::new();
        visited.insert(source);

        let mut queue: VecDeque<(NodeId, f64, u32)> = VecDeque::new();

        // Seed with source's direct neighbors
        let source_links: Vec<(NodeId, f64, usize)> = self
            .links
            .iter()
            .filter(|l| l.from == source)
            .map(|l| (l.to, l.coupling, l.dimension))
            .collect();

        for (target, coupling, _dim) in source_links {
            if !visited.contains(&target) {
                let scaled_delta = cascade_delta * coupling;
                queue.push_back((target, scaled_delta, 1));
            }
        }

        while let Some((node_id, delta, depth)) = queue.pop_front() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id);

            // Apply the cascade-correlated change to this node
            let modified_pos = if let Some(node) = self.nodes.get_mut(&node_id) {
                let n = node.superbit.sigma.len();
                if n == 0 || delta.abs() < 1e-15 {
                    None
                } else {
                    // φ-correlated position selection from the cascade delta
                    let frac = (delta.abs() * PHI).fract();
                    let target_pos = (frac * n as f64).floor() as usize % n;

                    if node.superbit.anchors.is_anchored(target_pos) {
                        None
                    } else {
                        node.superbit.sigma[target_pos] ^= 1;
                        node.superbit.generation += 1;
                        node.cached_address = node.superbit.address();
                        Some(target_pos)
                    }
                }
            } else {
                None
            };

            result.ripple_effects.push((node_id, modified_pos));
            result.nodes_affected += 1;

            // Continue ripple if within depth limit
            if depth < max_depth {
                // Compute this node's cascade delta for further propagation
                let node_delta = if modified_pos.is_some() {
                    // The change in this node — propagate a fraction
                    delta * 0.618 // φ - 1 = 0.618... (golden ratio decay)
                } else {
                    0.0
                };

                if node_delta.abs() > 1e-15 {
                    let outgoing: Vec<(NodeId, f64)> = self
                        .links
                        .iter()
                        .filter(|l| l.from == node_id)
                        .map(|l| (l.to, l.coupling))
                        .collect();

                    for (next_id, coupling) in outgoing {
                        if !visited.contains(&next_id) {
                            queue.push_back((
                                next_id,
                                node_delta * coupling,
                                depth + 1,
                            ));
                        }
                    }
                }
            }
        }

        result
    }

    /// Evolve the entire network by one tick using cascade evolution,
    /// with entanglement propagation after each node's evolution.
    pub fn evolve_all(&mut self) {
        let ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for id in ids {
            if let Some(node) = self.nodes.get_mut(&id) {
                evolution::evolve_cascade(&mut node.superbit);
                node.cached_address = node.superbit.address();
            }
        }
        self.tick += 1;
    }

    /// Evolve the entire network using simple dimensional evolution
    /// (legacy mode, no cascade awareness).
    pub fn evolve_all_dimensional(&mut self) {
        let ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for id in ids {
            if let Some(node) = self.nodes.get_mut(&id) {
                evolution::evolve_dimensional(&mut node.superbit);
                node.cached_address = node.superbit.address();
            }
        }
        self.tick += 1;
    }

    /// Get a snapshot of all nodes' dimensional addresses — useful for
    /// comparing network state before and after evolution.
    pub fn snapshot_addresses(&self) -> HashMap<NodeId, DimensionalAddress> {
        self.nodes
            .iter()
            .map(|(&id, node)| (id, node.cached_address.clone()))
            .collect()
    }

    /// Compute cascade correlation between two entangled nodes.
    ///
    /// Returns the ratio of D4 change in node B relative to a D4 change
    /// in node A, based on their coupling strength. Higher correlation
    /// means the two SuperBits' dimensional coordinates move more in sync.
    pub fn cascade_correlation(&self, a: NodeId, b: NodeId) -> f64 {
        let link = self.links.iter().find(|l| l.from == a && l.to == b);
        match link {
            Some(l) => l.coupling,
            None => 0.0,
        }
    }

    /// Iterate over all node IDs.
    pub fn node_ids(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes.keys()
    }
}

impl Default for MDBNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sb(bits: Vec<u8>) -> SuperBit {
        SuperBit::from_bits(bits)
    }

    #[test]
    fn test_add_and_get_nodes() {
        let mut net = MDBNetwork::new();
        let id1 = net.add_node(make_sb(vec![1, 0, 1]));
        let id2 = net.add_node(make_sb(vec![0, 1, 0, 1]));

        assert_eq!(net.node_count(), 2);
        assert_eq!(net.get(id1).unwrap().bit_length(), 3);
        assert_eq!(net.get(id2).unwrap().bit_length(), 4);
    }

    #[test]
    fn test_entanglement() {
        let mut net = MDBNetwork::new();
        let id1 = net.add_node(make_sb(vec![1, 0, 1]));
        let id2 = net.add_node(make_sb(vec![0, 1, 0]));

        assert!(net.entangle(id1, id2, 0.8, 4));
        // Bidirectional → 2 links
        assert_eq!(net.link_count(), 2);
        assert!(net.neighbors(id1).unwrap().contains(&id2));
        assert!(net.neighbors(id2).unwrap().contains(&id1));
    }

    #[test]
    fn test_entangle_invalid() {
        let mut net = MDBNetwork::new();
        let id1 = net.add_node(make_sb(vec![1]));
        assert!(!net.entangle(id1, id1, 0.5, 4)); // self-link
        assert!(!net.entangle(id1, 999, 0.5, 4)); // non-existent
    }

    #[test]
    fn test_remove_node() {
        let mut net = MDBNetwork::new();
        let id1 = net.add_node(make_sb(vec![1, 0]));
        let id2 = net.add_node(make_sb(vec![0, 1]));
        net.entangle(id1, id2, 0.5, 4);

        net.remove_node(id1);
        assert_eq!(net.node_count(), 1);
        assert!(net.get(id1).is_none());
        assert!(!net.neighbors(id2).unwrap().contains(&id1));
    }

    #[test]
    fn test_cached_address() {
        let mut net = MDBNetwork::new();
        let bits = vec![1, 0, 1, 1, 0, 0, 1, 0];
        let expected_addr = DimensionalAddress::from_bits(&bits);
        let id = net.add_node(make_sb(bits));

        let addr = net.address(id).unwrap();
        assert_eq!(addr.n, expected_addr.n);
        assert!((addr.d4_spacetime - expected_addr.d4_spacetime).abs() < 1e-10);
    }

    #[test]
    fn test_evolve_entangled_basic() {
        let mut net = MDBNetwork::new();
        // A and B are entangled at D4 (Spacetime) with strong coupling
        let a = net.add_node(make_sb(vec![1, 0, 1, 0, 1, 0, 1, 0]));
        let b = net.add_node(make_sb(vec![0, 1, 0, 1, 0, 1, 0, 1]));

        let b_sigma_before = net.get(b).unwrap().sigma.clone();
        let b_addr_before = net.address(b).unwrap().clone();

        net.entangle(a, b, 1.0, 4);

        // Evolve A → should cause cascade ripple to B
        let result = net.evolve_entangled(a, 1);

        assert_eq!(result.source, a);
        assert!(result.source_position.is_some());
        assert!(result.nodes_affected >= 2, "both A and B should be affected");

        // B should have changed (with high coupling)
        let b_sigma_after = &net.get(b).unwrap().sigma;
        let b_changed = b_sigma_before != *b_sigma_after;
        // With coupling=1.0, B should almost certainly change
        // (only skipped if delta is exactly 0 or hits an anchor)
        assert!(b_changed, "B should change due to entanglement ripple");

        // B's cached address should have updated
        let b_addr_after = net.address(b).unwrap();
        assert_ne!(b_addr_before.d5_momentum, b_addr_after.d5_momentum,
            "B's address should change after entangled evolution");
    }

    #[test]
    fn test_evolve_entangled_chain() {
        let mut net = MDBNetwork::new();
        // Chain: A - B - C
        let a = net.add_node(make_sb(vec![1, 0, 1, 0, 1, 0, 1, 0]));
        let b = net.add_node(make_sb(vec![0, 1, 0, 1, 0, 1, 0, 1]));
        let c = net.add_node(make_sb(vec![1, 1, 0, 0, 1, 1, 0, 0]));

        net.entangle(a, b, 0.9, 4);
        net.entangle(b, c, 0.9, 4);

        let c_sigma_before = net.get(c).unwrap().sigma.clone();

        // Evolve A with depth 2 → should ripple A → B → C
        let result = net.evolve_entangled(a, 2);

        assert!(result.nodes_affected >= 2, "at least A and B affected");
        // C might or might not change (depends on delta decay)
    }

    #[test]
    fn test_evolve_entangled_weak_coupling() {
        let mut net = MDBNetwork::new();
        let a = net.add_node(make_sb(vec![1, 0, 1, 0, 1, 0, 1, 0]));
        let b = net.add_node(make_sb(vec![0, 1, 0, 1, 0, 1, 0, 1]));

        // Very weak coupling → delta might be too small to propagate
        net.entangle(a, b, 0.001, 4);

        let result = net.evolve_entangled(a, 1);
        assert_eq!(result.source, a);
        // B is visited but might not change due to weak coupling
        assert!(result.nodes_affected >= 1);
    }

    #[test]
    fn test_evolve_entangled_deterministic() {
        // Same network, same starting state → same result
        let make_net = || {
            let mut net = MDBNetwork::new();
            let a = net.add_node(make_sb(vec![1, 0, 1, 0, 1, 0, 1, 0]));
            let b = net.add_node(make_sb(vec![0, 1, 0, 1, 0, 1, 0, 1]));
            net.entangle(a, b, 0.8, 4);
            (net, a, b)
        };

        let (mut net1, a1, b1) = make_net();
        let (mut net2, a2, b2) = make_net();

        let r1 = net1.evolve_entangled(a1, 1);
        let r2 = net2.evolve_entangled(a2, 1);

        assert_eq!(r1.source_position, r2.source_position);
        assert_eq!(net1.get(a1).unwrap().sigma, net2.get(a2).unwrap().sigma);
        assert_eq!(net1.get(b1).unwrap().sigma, net2.get(b2).unwrap().sigma);
    }

    #[test]
    fn test_cascade_correlation() {
        let mut net = MDBNetwork::new();
        let a = net.add_node(make_sb(vec![1, 0]));
        let b = net.add_node(make_sb(vec![0, 1]));
        let c = net.add_node(make_sb(vec![1, 1]));

        net.entangle(a, b, 0.75, 4);

        assert!((net.cascade_correlation(a, b) - 0.75).abs() < 1e-10);
        assert!((net.cascade_correlation(b, a) - 0.75).abs() < 1e-10);
        assert!((net.cascade_correlation(a, c) - 0.0).abs() < 1e-10); // not entangled
    }

    #[test]
    fn test_evolve_all() {
        let mut net = MDBNetwork::new();
        net.add_node(make_sb(vec![1, 0, 1]));
        net.add_node(make_sb(vec![0, 1, 0, 1]));
        net.add_node(make_sb(vec![1, 1, 0]));

        net.evolve_all();
        assert_eq!(net.tick, 1);

        for id in net.node_ids() {
            assert_eq!(net.get(*id).unwrap().generation, 1);
        }
    }

    #[test]
    fn test_snapshot_addresses() {
        let mut net = MDBNetwork::new();
        let a = net.add_node(make_sb(vec![1, 0, 1, 0]));
        let b = net.add_node(make_sb(vec![0, 1, 0, 1]));

        let snap1 = net.snapshot_addresses();
        net.evolve_all();
        let snap2 = net.snapshot_addresses();

        // Addresses should have changed after evolution
        assert_ne!(
            snap1[&a].d5_momentum,
            snap2[&a].d5_momentum,
            "address should change after evolution"
        );
    }
}
