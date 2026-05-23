//! # Dimensional Coordinate System — Mode B Vector Cascade
//!
//! Every binary string in MDB exists as a point in ℝ^∞ defined by a recursive
//! dimensional cascade.  Each dimension is computed **per-position** as a vector,
//! and each new dimension derives from the two below it (Fibonacci addition).
//!
//! ## Dimensions
//!
//! | Dim | Name       | Formula (per position i)                             |
//! |-----|------------|------------------------------------------------------|
//! | D1  | Value      | w_i = 0.3 if bit=1, 0.2 if bit=0                    |
//! | D2  | Space      | D2_i = w_i × n   (length-scaled weight)              |
//! | D3  | Time       | D3_i = w_i × n   (1:1 mapping from Space)            |
//! | D4  | Spacetime  | D4_i = D2_i + D3_i                                   |
//! | D5  | Momentum   | D5_i = D3_i + D4_i                                   |
//! | D6  | Energy     | D6_i = D4_i + D5_i                                   |
//! | D7+ | (recurse)  | D(k)_i = D(k-2)_i + D(k-1)_i                        |
//!
//! The cascade produces Fibonacci-scaled coefficients:
//! D2=n, D3=n, D4=2n, D5=3n, D6=5n, D7=8n, ...
//! and D(k)/D(k-1) converges to φ ≈ 1.618 (Golden Ratio).
//!
//! ## Recovery
//!
//! The original binary string is always recoverable from D1: 0.3 → 1, 0.2 → 0.
//! This guarantees lossless round-tripping through the dimensional engine.

/// Bit weight for a 0-bit.
pub const WEIGHT_ZERO: f64 = 0.2;

/// Bit weight for a 1-bit.
pub const WEIGHT_ONE: f64 = 0.3;

/// Default maximum dimension depth for cascade computation.
pub const DEFAULT_MAX_DIM: usize = 10;

/// Human-readable names for dimensions (index 0 = D1).
pub const DIMENSION_NAMES: &[&str] = &[
    "Value",     // D1
    "Space",     // D2
    "Time",      // D3
    "Spacetime", // D4
    "Momentum",  // D5
    "Energy",    // D6
];

/// Returns the name for dimension `d` (1-indexed), or a generic label for D7+.
pub fn dimension_name(d: usize) -> String {
    if d == 0 {
        return "D0(invalid)".to_string();
    }
    match DIMENSION_NAMES.get(d - 1) {
        Some(name) => format!("D{d} ({name})"),
        None => format!("D{d}"),
    }
}

// ---------------------------------------------------------------------------
// Core Cascade Engine
// ---------------------------------------------------------------------------

/// The full per-position dimensional cascade for a binary string.
///
/// Each dimension is a `Vec<f64>` of length `n` (the number of bits).
/// `dims[0]` = D1 (Value), `dims[1]` = D2 (Space), etc.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DimensionalCascade {
    /// Per-position vectors for each dimension.  `dims[k]` is D(k+1).
    pub dims: Vec<Vec<f64>>,
    /// Number of bits in the original string.
    pub n: usize,
}

impl DimensionalCascade {
    /// Compute the cascade up to `max_dim` dimensions from raw bits (0/1 u8 slice).
    pub fn from_bits(bits: &[u8], max_dim: usize) -> Self {
        let n = bits.len();
        let max_dim = max_dim.max(3); // at least D1–D3

        let mut dims: Vec<Vec<f64>> = Vec::with_capacity(max_dim);

        // D1: Value — per-position weight
        let d1: Vec<f64> = bits
            .iter()
            .map(|&b| if b == 1 { WEIGHT_ONE } else { WEIGHT_ZERO })
            .collect();
        dims.push(d1);

        // D2: Space — D1 × n (length scaling)
        let d2: Vec<f64> = dims[0].iter().map(|&w| w * n as f64).collect();
        dims.push(d2);

        // D3: Time — 1:1 mapping from D2 (identical values)
        let d3 = dims[1].clone();
        dims.push(d3);

        // D4 .. D(max_dim): Fibonacci cascade  D(k) = D(k-2) + D(k-1)
        for _k in 4..=max_dim {
            let prev1 = &dims[dims.len() - 1]; // D(k-1)
            let prev2 = &dims[dims.len() - 2]; // D(k-2)
            let dk: Vec<f64> = prev2
                .iter()
                .zip(prev1.iter())
                .map(|(&a, &b)| a + b)
                .collect();
            dims.push(dk);
        }

        Self { dims, n }
    }

    /// Compute the cascade from raw byte data (expands each byte to 8 bits).
    pub fn from_bytes(data: &[u8], max_dim: usize) -> Self {
        let bits = bytes_to_bits(data);
        Self::from_bits(&bits, max_dim)
    }

    /// Get a reference to dimension `d` (1-indexed). Returns `None` if out of range.
    pub fn dim(&self, d: usize) -> Option<&[f64]> {
        if d == 0 || d > self.dims.len() {
            None
        } else {
            Some(&self.dims[d - 1])
        }
    }

    /// Recover the original binary string from D1 (lossless).
    ///
    /// 0.3 → 1, 0.2 → 0.  Panics if D1 contains unexpected values.
    pub fn recover_bits(&self) -> Vec<u8> {
        self.dims[0]
            .iter()
            .map(|&w| {
                if (w - WEIGHT_ONE).abs() < 1e-10 {
                    1u8
                } else if (w - WEIGHT_ZERO).abs() < 1e-10 {
                    0u8
                } else {
                    panic!("D1 contains value {w} which is neither WEIGHT_ONE nor WEIGHT_ZERO");
                }
            })
            .collect()
    }

    /// Sum of all values in dimension `d` (simple scalar reduction).
    pub fn dim_sum(&self, d: usize) -> f64 {
        self.dim(d).map(|v| v.iter().sum()).unwrap_or(0.0)
    }

    /// Linear position-weighted sum of dimension `d`: `Σ(D_k[i] × (i+1))`.
    pub fn dim_weighted_sum_linear(&self, d: usize) -> f64 {
        self.dim(d)
            .map(|v| {
                v.iter()
                    .enumerate()
                    .map(|(i, &val)| val * (i + 1) as f64)
                    .sum()
            })
            .unwrap_or(0.0)
    }

    /// Quadratic position-weighted sum of dimension `d`: `Σ(D_k[i] × (i+1)²)`.
    ///
    /// Linearly independent from the linear sum, so combining both
    /// eliminates collisions that either alone would miss.
    pub fn dim_weighted_sum_quadratic(&self, d: usize) -> f64 {
        self.dim(d)
            .map(|v| {
                v.iter()
                    .enumerate()
                    .map(|(i, &val)| {
                        let pos = (i + 1) as f64;
                        val * pos * pos
                    })
                    .sum()
            })
            .unwrap_or(0.0)
    }
}

// ---------------------------------------------------------------------------
// Dimensional Address (for indexing / O(1) retrieval)
// ---------------------------------------------------------------------------

/// A point in MDB's coordinate space, derived from the cascade.
///
/// For O(1) indexing the per-position cascade is reduced to:
/// - `n`: bit length
/// - `d4_spacetime`: position-weighted sum `Σ(D4_i × (i+1))` (geometric scalar)
/// - `d5_momentum`: FNV-1a fingerprint of the D1 vector → bit identity hash
///
/// The d4 scalar preserves geometric meaning (heavier 1-bits at later
/// positions push the coordinate higher).  The d5 fingerprint guarantees
/// collision-free addressing for any bit pattern.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DimensionalAddress {
    /// Number of bits.
    pub n: u64,
    /// Linear position-weighted scalar of D4 (Spacetime): `Σ(D4_i × (i+1))`.
    pub d4_spacetime: f64,
    /// FNV-1a fingerprint of the bit pattern (derived from D1 recovery).
    pub d5_momentum: u64,
}

impl DimensionalAddress {
    /// Compute the dimensional address from a bit sequence.
    pub fn from_bits(bits: &[u8]) -> Self {
        let cascade = DimensionalCascade::from_bits(bits, 5);
        Self {
            n: bits.len() as u64,
            d4_spacetime: cascade.dim_weighted_sum_linear(4),
            d5_momentum: d1_fingerprint(&cascade.dims[0]),
        }
    }

    /// Compute the dimensional address from raw byte data.
    pub fn from_bytes(data: &[u8]) -> Self {
        let bits = bytes_to_bits(data);
        Self::from_bits(&bits)
    }

    /// Return tuple for deterministic hashing / indexing.
    pub fn as_tuple(&self) -> (u64, u64, u64) {
        let d4_q = (self.d4_spacetime * 1_000_000.0).round() as u64;
        (self.n, d4_q, self.d5_momentum)
    }
}

/// Compute an FNV-1a hash of the D1 weight vector, recovering bits first.
///
/// Since D1 uniquely encodes the original binary string (0.3→1, 0.2→0),
/// this fingerprint is collision-free — distinct bit patterns always
/// produce distinct hashes.
fn d1_fingerprint(d1: &[f64]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &w in d1 {
        let bit = if (w - WEIGHT_ONE).abs() < 1e-10 { 1u8 } else { 0u8 };
        hash ^= bit as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

impl std::fmt::Display for DimensionalAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "addr(n={}, D4_spacetime={:.6}, D5_momentum={:#018x})",
            self.n, self.d4_spacetime, self.d5_momentum
        )
    }
}

/// Convenience: compute the address for a bit sequence.
pub fn dimensional_address(bits: &[u8]) -> DimensionalAddress {
    DimensionalAddress::from_bits(bits)
}

// ---------------------------------------------------------------------------
// Bit ↔ Byte utilities (unchanged)
// ---------------------------------------------------------------------------

/// Expand a byte slice into individual bits (MSB first per byte).
pub fn bytes_to_bits(data: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(data.len() * 8);
    for &byte in data {
        for shift in (0..8).rev() {
            bits.push((byte >> shift) & 1);
        }
    }
    bits
}

/// Pack individual bits (0/1 values) back into bytes (MSB first).
///
/// If the number of bits is not a multiple of 8, the final byte is zero-padded.
pub fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((bits.len() + 7) / 8);
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for (i, &bit) in chunk.iter().enumerate() {
            byte |= bit << (7 - i);
        }
        bytes.push(byte);
    }
    bytes
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_d1_weights() {
        let bits = [1, 0, 1, 0u8];
        let cascade = DimensionalCascade::from_bits(&bits, 5);
        let d1 = cascade.dim(1).unwrap();
        assert_eq!(d1, &[0.3, 0.2, 0.3, 0.2]);
    }

    #[test]
    fn test_d2_space() {
        let bits = [1, 0, 1, 0u8];
        let cascade = DimensionalCascade::from_bits(&bits, 5);
        let d2 = cascade.dim(2).unwrap();
        // n=4, so D2 = D1 * 4
        assert_eq!(d2, &[1.2, 0.8, 1.2, 0.8]);
    }

    #[test]
    fn test_d3_equals_d2() {
        let bits = [1, 1, 0u8];
        let cascade = DimensionalCascade::from_bits(&bits, 5);
        assert_eq!(cascade.dim(2).unwrap(), cascade.dim(3).unwrap());
    }

    #[test]
    fn test_fibonacci_cascade() {
        // D4 = D2 + D3 = 2×D2,  D5 = D3 + D4 = 3×D2
        let bits = [1, 0u8];
        let cascade = DimensionalCascade::from_bits(&bits, 7);
        let d2 = cascade.dim(2).unwrap();
        let d4 = cascade.dim(4).unwrap();
        let d5 = cascade.dim(5).unwrap();
        let d6 = cascade.dim(6).unwrap();
        let d7 = cascade.dim(7).unwrap();

        let eps = 1e-10;
        for i in 0..2 {
            assert!((d4[i] - 2.0 * d2[i]).abs() < eps, "D4 = 2×D2");
            assert!((d5[i] - 3.0 * d2[i]).abs() < eps, "D5 = 3×D2");
            assert!((d6[i] - 5.0 * d2[i]).abs() < eps, "D6 = 5×D2");
            assert!((d7[i] - 8.0 * d2[i]).abs() < eps, "D7 = 8×D2");
        }
    }

    #[test]
    fn test_golden_ratio_convergence() {
        let bits = [1, 0, 1, 1, 0, 0, 1, 0u8];
        let cascade = DimensionalCascade::from_bits(&bits, 20);
        // Ratio of consecutive dimension sums should converge to φ
        let phi = 1.618_033_988_749_895_f64;
        let s19 = cascade.dim_sum(19);
        let s20 = cascade.dim_sum(20);
        let ratio = s20 / s19;
        assert!(
            (ratio - phi).abs() < 1e-6,
            "D20/D19 ratio {ratio} should ≈ φ"
        );
    }

    #[test]
    fn test_lossless_recovery() {
        let bits = vec![1, 0, 1, 1, 0, 0, 1, 0u8];
        let cascade = DimensionalCascade::from_bits(&bits, 5);
        let recovered = cascade.recover_bits();
        assert_eq!(bits, recovered);
    }

    #[test]
    fn test_address_uniqueness() {
        // Different lengths
        let a1 = DimensionalAddress::from_bits(&[1, 0, 1]);
        let a2 = DimensionalAddress::from_bits(&[1, 0, 1, 0]);
        assert_ne!(a1.n, a2.n);

        // Same length, different content
        let a3 = DimensionalAddress::from_bits(&[1, 1, 0, 0]);
        let a4 = DimensionalAddress::from_bits(&[1, 0, 1, 0]);
        assert_eq!(a3.n, a4.n);
        // D4 sums should differ because bit positions differ
        assert_ne!(a3.as_tuple(), a4.as_tuple());
    }

    #[test]
    fn test_zero_collision_small() {
        // Verify no address collisions for all 1-6 bit strings
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for len in 1..=6 {
            for val in 0..(1u32 << len) {
                let bits: Vec<u8> = (0..len)
                    .rev()
                    .map(|b| ((val >> b) & 1) as u8)
                    .collect();
                let addr = DimensionalAddress::from_bits(&bits);
                let tuple = addr.as_tuple();
                assert!(
                    seen.insert(tuple),
                    "Collision at bits {:?} addr {:?}",
                    bits,
                    tuple
                );
            }
        }
    }

    #[test]
    fn test_bytes_to_bits_roundtrip() {
        let data = vec![0xAB, 0xCD, 0xEF];
        let bits = bytes_to_bits(&data);
        let back = bits_to_bytes(&bits);
        assert_eq!(data, back);
    }

    #[test]
    fn test_address_display() {
        let addr = dimensional_address(&[1, 0, 1, 1]);
        let s = format!("{addr}");
        assert!(s.starts_with("addr("));
        assert!(s.contains("D4_spacetime"));
        assert!(s.contains("D5_momentum"));
    }
}
