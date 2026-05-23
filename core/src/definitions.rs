//! # DefinitionsList — Immutable Anchor Positions
//!
//! The DefinitionsList manages **immutable anchor positions** within a binary
//! string. These positions are protected from modification during dimensional
//! evolution, preserving the string's structural identity across transformations.
//!
//! Think of it as the string's fundamental rules — the bits that define what
//! the SuperBit *is* and must never change, even as the SuperBit evolves and
//! learns.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A set of immutable anchor positions within a binary string.
///
/// Positions in this set are protected from all mutation operations
/// (dimensional evolution, learning reweight re-encoding, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefinitionsList {
    /// Sorted set of protected bit positions.
    anchors: BTreeSet<usize>,
}

impl DefinitionsList {
    /// Create an empty definitions list.
    pub fn new() -> Self {
        Self {
            anchors: BTreeSet::new(),
        }
    }

    /// Create a definitions list from an iterator of positions.
    pub fn from_positions(positions: impl IntoIterator<Item = usize>) -> Self {
        Self {
            anchors: positions.into_iter().collect(),
        }
    }

    /// Add an anchor position.
    pub fn add(&mut self, position: usize) {
        self.anchors.insert(position);
    }

    /// Remove an anchor position.
    pub fn remove(&mut self, position: usize) -> bool {
        self.anchors.remove(&position)
    }

    /// Check if a position is anchored (protected).
    pub fn is_anchored(&self, position: usize) -> bool {
        self.anchors.contains(&position)
    }

    /// Get the number of anchored positions.
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Check if there are no anchors.
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Iterate over all anchored positions in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &usize> {
        self.anchors.iter()
    }

    /// Get all anchor positions as a sorted vector.
    pub fn positions(&self) -> Vec<usize> {
        self.anchors.iter().copied().collect()
    }

    /// Encode the definitions list into a binary representation.
    ///
    /// Format: [count: 4 bytes BE][pos1: 4 bytes BE][pos2: 4 bytes BE]...
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + self.anchors.len() * 4);
        out.extend_from_slice(&(self.anchors.len() as u32).to_be_bytes());
        for &pos in &self.anchors {
            out.extend_from_slice(&(pos as u32).to_be_bytes());
        }
        out
    }

    /// Decode a definitions list from its binary representation.
    pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() < 4 {
            return None;
        }
        let count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let total_len = 4 + count * 4;
        if data.len() < total_len {
            return None;
        }
        let mut anchors = BTreeSet::new();
        for i in 0..count {
            let offset = 4 + i * 4;
            let pos = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            anchors.insert(pos);
        }
        Some((Self { anchors }, total_len))
    }
}

impl Default for DefinitionsList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut defs = DefinitionsList::new();
        assert!(defs.is_empty());

        defs.add(5);
        defs.add(10);
        defs.add(0);
        assert_eq!(defs.len(), 3);
        assert!(defs.is_anchored(5));
        assert!(!defs.is_anchored(6));
        assert_eq!(defs.positions(), vec![0, 5, 10]);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let defs = DefinitionsList::from_positions(vec![3, 7, 15, 42, 100]);
        let encoded = defs.encode();
        let (decoded, bytes_read) = DefinitionsList::decode(&encoded).unwrap();
        assert_eq!(defs, decoded);
        assert_eq!(bytes_read, encoded.len());
    }

    #[test]
    fn test_from_positions() {
        let defs = DefinitionsList::from_positions(vec![10, 5, 10, 3]); // duplicates removed
        assert_eq!(defs.len(), 3);
        assert_eq!(defs.positions(), vec![3, 5, 10]);
    }
}
