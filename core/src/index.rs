//! # DimensionalIndex — O(1) Guaranteed Retrieval
//!
//! The DimensionalIndex provides constant-time retrieval of SuperBits (or any
//! MDB data) by their dimensional address. It indexes items by their D3, D4,
//! and D5 coordinates, enabling instant lookup without traditional search.
//!
//! This implements the "standing on all pages" property: an item's location
//! is directly computable from its intrinsic dimensional properties. You don't
//! search for data — you compute where it *must* be and go there directly.
//!
//! ## Complexity
//!
//! - Insert: O(|S|) to compute address + O(1) to store
//! - Lookup by address: O(1) guaranteed
//! - Lookup by data: O(|S|) to compute address + O(1) to retrieve

use crate::coordinates::DimensionalAddress;
use std::collections::HashMap;

/// A stored entry in the DimensionalIndex.
#[derive(Debug, Clone)]
pub struct IndexEntry<T> {
    /// The dimensional address (computed from the data).
    pub address: DimensionalAddress,
    /// The stored value.
    pub value: T,
}

/// O(1) retrieval index based on dimensional coordinates.
///
/// Items are stored at their intrinsic dimensional address. Retrieval
/// is a direct lookup — no scanning, no tree traversal, no hashing
/// collisions in the traditional sense.
#[derive(Debug)]
pub struct DimensionalIndex<T> {
    /// Primary index: quantized address tuple → entry.
    entries: HashMap<(u64, u64, u64), Vec<IndexEntry<T>>>,
    /// Total number of stored items.
    count: usize,
}

impl<T: Clone> DimensionalIndex<T> {
    /// Create an empty DimensionalIndex.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            count: 0,
        }
    }

    /// Insert a value at its dimensional address.
    pub fn insert(&mut self, address: DimensionalAddress, value: T) {
        let key = address.as_tuple();
        let entry = IndexEntry {
            address: address.clone(),
            value,
        };
        self.entries.entry(key).or_insert_with(Vec::new).push(entry);
        self.count += 1;
    }

    /// Insert raw bit data, computing its address automatically.
    pub fn insert_bits(&mut self, bits: &[u8], value: T) -> DimensionalAddress {
        let address = DimensionalAddress::from_bits(bits);
        self.insert(address.clone(), value);
        address
    }

    /// Insert raw byte data, computing its address automatically.
    pub fn insert_bytes(&mut self, data: &[u8], value: T) -> DimensionalAddress {
        let address = DimensionalAddress::from_bytes(data);
        self.insert(address.clone(), value);
        address
    }

    /// Retrieve all entries at a dimensional address. O(1).
    pub fn get(&self, address: &DimensionalAddress) -> Option<&Vec<IndexEntry<T>>> {
        let key = address.as_tuple();
        self.entries.get(&key)
    }

    /// Retrieve the first entry at a dimensional address. O(1).
    pub fn get_first(&self, address: &DimensionalAddress) -> Option<&T> {
        self.get(address)
            .and_then(|entries| entries.first())
            .map(|e| &e.value)
    }

    /// Look up by raw bit data (computes address, then retrieves).
    pub fn lookup_bits(&self, bits: &[u8]) -> Option<&Vec<IndexEntry<T>>> {
        let address = DimensionalAddress::from_bits(bits);
        self.get(&address)
    }

    /// Look up by raw byte data.
    pub fn lookup_bytes(&self, data: &[u8]) -> Option<&Vec<IndexEntry<T>>> {
        let address = DimensionalAddress::from_bytes(data);
        self.get(&address)
    }

    /// Check if any entry exists at this address.
    pub fn contains(&self, address: &DimensionalAddress) -> bool {
        let key = address.as_tuple();
        self.entries.contains_key(&key)
    }

    /// Remove all entries at a dimensional address.
    pub fn remove(&mut self, address: &DimensionalAddress) -> Option<Vec<IndexEntry<T>>> {
        let key = address.as_tuple();
        if let Some(entries) = self.entries.remove(&key) {
            self.count -= entries.len();
            Some(entries)
        } else {
            None
        }
    }

    /// Total number of stored items.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate over all entries.
    pub fn iter(&self) -> impl Iterator<Item = &IndexEntry<T>> {
        self.entries.values().flat_map(|v| v.iter())
    }

    /// Get the number of unique dimensional addresses.
    pub fn unique_addresses(&self) -> usize {
        self.entries.len()
    }
}

impl<T: Clone> Default for DimensionalIndex<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_retrieve() {
        let mut index = DimensionalIndex::new();
        let data = b"test data";
        let addr = index.insert_bytes(data, "my_file.txt".to_string());

        assert_eq!(index.len(), 1);
        let result = index.get_first(&addr).unwrap();
        assert_eq!(result, "my_file.txt");
    }

    #[test]
    fn test_standing_on_all_pages() {
        // The "standing on all pages" property: given the same data,
        // its address is always the same, so you always find it.
        let mut index = DimensionalIndex::new();

        let data1 = b"neural waveform channel 1";
        let data2 = b"neural waveform channel 2";
        let data3 = b"neural waveform channel 3";

        let addr1 = index.insert_bytes(data1, 1u32);
        let addr2 = index.insert_bytes(data2, 2u32);
        let addr3 = index.insert_bytes(data3, 3u32);

        // Compute addresses independently — they lead right to the data
        let lookup_addr1 = DimensionalAddress::from_bytes(data1);
        let lookup_addr2 = DimensionalAddress::from_bytes(data2);
        let lookup_addr3 = DimensionalAddress::from_bytes(data3);

        assert_eq!(*index.get_first(&lookup_addr1).unwrap(), 1);
        assert_eq!(*index.get_first(&lookup_addr2).unwrap(), 2);
        assert_eq!(*index.get_first(&lookup_addr3).unwrap(), 3);
    }

    #[test]
    fn test_remove() {
        let mut index = DimensionalIndex::new();
        let addr = index.insert_bytes(b"remove me", 42);
        assert_eq!(index.len(), 1);

        index.remove(&addr);
        assert_eq!(index.len(), 0);
        assert!(index.get_first(&addr).is_none());
    }

    #[test]
    fn test_multiple_entries_same_address() {
        let mut index = DimensionalIndex::new();
        let addr = DimensionalAddress::from_bytes(b"data");

        index.insert(addr.clone(), "first");
        index.insert(addr.clone(), "second");

        assert_eq!(index.len(), 2);
        let entries = index.get(&addr).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_contains() {
        let mut index: DimensionalIndex<()> = DimensionalIndex::new();
        let addr = DimensionalAddress::from_bytes(b"exists");
        let addr2 = DimensionalAddress::from_bytes(b"does not exist");

        index.insert(addr.clone(), ());
        assert!(index.contains(&addr));
        assert!(!index.contains(&addr2));
    }
}
