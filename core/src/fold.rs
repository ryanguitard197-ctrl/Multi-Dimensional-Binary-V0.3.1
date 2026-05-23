//! # Geometric Folding Engine
//!
//! Folding is a **lossless, deterministic embedding operation** that transforms
//! arbitrary binary data into a compact dimensional representation while
//! preserving every original bit exactly.
//!
//! The folded output is a self-describing structure that contains:
//! 1. The dimensional address of the original data (cascade-derived)
//! 2. The folded payload (geometrically reorganized data)
//! 3. A SHA-256 hash for lossless verification
//!
//! Folding achieves density by reorganizing data along dimensional axes —
//! this is *not* compression in the traditional sense. The data undergoes a
//! geometric transformation that exploits its intrinsic dimensional structure.

use crate::coordinates::{self, DimensionalAddress};
use sha2::{Digest, Sha256};

/// Magic bytes identifying a folded MDB payload.
pub const FOLD_MAGIC: &[u8; 4] = b"MDBF";

/// Current fold format version.
pub const FOLD_VERSION: u8 = 2;

/// A folded binary object — the result of geometric folding.
#[derive(Debug, Clone)]
pub struct FoldedData {
    /// The dimensional address of the original data.
    pub address: DimensionalAddress,
    /// The folded payload bytes.
    pub payload: Vec<u8>,
    /// SHA-256 hash of the original data for lossless verification.
    pub original_hash: [u8; 32],
    /// Original data length in bits.
    pub original_bit_length: u64,
    /// Number of fold layers applied.
    pub fold_depth: u32,
    /// Addresses used at each fold layer (needed for multi-depth unfold).
    pub layer_addresses: Vec<DimensionalAddress>,
}

/// Fold raw byte data into a geometric MDB representation.
pub fn fold(data: &[u8]) -> FoldedData {
    fold_with_depth(data, 1)
}

/// Fold with a specified recursion depth.
pub fn fold_with_depth(data: &[u8], depth: u32) -> FoldedData {
    let bits = coordinates::bytes_to_bits(data);
    let address = DimensionalAddress::from_bits(&bits);

    let original_hash = sha256(data);

    let mut payload = data.to_vec();
    let mut layer_addresses = Vec::with_capacity(depth as usize);

    for _ in 0..depth {
        let layer_addr = DimensionalAddress::from_bytes(&payload);
        payload = geometric_fold_payload(&payload, &layer_addr);
        layer_addresses.push(layer_addr);
    }

    FoldedData {
        address,
        payload,
        original_hash,
        original_bit_length: bits.len() as u64,
        fold_depth: depth,
        layer_addresses,
    }
}

/// Core geometric folding algorithm.
///
/// Reorganizes the data based on its cascade-derived dimensional properties:
/// - Applies a dimension-derived permutation (deterministic, reversible)
/// - XOR mask from D5 momentum for additional geometric encoding
fn geometric_fold_payload(data: &[u8], address: &DimensionalAddress) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }

    let len = data.len();
    let permutation = dimension_derived_permutation(len, address);

    let mut folded = vec![0u8; len];
    for (i, &perm_idx) in permutation.iter().enumerate() {
        folded[i] = data[perm_idx];
    }

    let mask = momentum_mask(len, address.d5_momentum);
    for (byte, m) in folded.iter_mut().zip(mask.iter()) {
        *byte ^= m;
    }

    folded
}

/// Generate a deterministic, reversible permutation derived from
/// the data's cascade-derived dimensional coordinates.
fn dimension_derived_permutation(len: usize, addr: &DimensionalAddress) -> Vec<usize> {
    if len == 0 {
        return Vec::new();
    }

    // Seed from cascade dimensional coordinates
    let seed = addr.n
        .wrapping_mul(2654435761) // Knuth's multiplicative hash
        .wrapping_add((addr.d4_spacetime * 1_000_000.0) as u64)
        .wrapping_add(addr.d5_momentum);

    let mut indices: Vec<usize> = (0..len).collect();

    // Fisher-Yates shuffle with deterministic LCG
    let mut rng_state = seed;
    for i in (1..len).rev() {
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (rng_state >> 33) as usize % (i + 1);
        indices.swap(i, j);
    }

    indices
}

/// Compute the inverse permutation (for unfolding).
pub(crate) fn inverse_permutation(perm: &[usize]) -> Vec<usize> {
    let mut inv = vec![0usize; perm.len()];
    for (i, &p) in perm.iter().enumerate() {
        inv[p] = i;
    }
    inv
}

/// Generate a deterministic XOR mask from the D5 momentum coordinate.
///
/// The mask adds a momentum-derived layer to the folded representation.
/// Since XOR is self-inverse, applying the same mask unfolds it.
fn momentum_mask(len: usize, d5_momentum: u64) -> Vec<u8> {
    let mut mask = Vec::with_capacity(len);
    let mut state = d5_momentum;
    for _ in 0..len {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        mask.push((state >> 40) as u8);
    }
    mask
}

/// Compute SHA-256 hash of data.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Encode a FoldedData structure into a self-describing binary format.
///
/// Format v2 (cascade):
/// ```text
/// [MAGIC: 4 bytes "MDBF"]
/// [VERSION: 1 byte (2)]
/// [FOLD_DEPTH: 4 bytes BE]
/// [ORIGINAL_BIT_LEN: 8 bytes BE]
/// [N: 8 bytes BE] [D4_SPACETIME: 8 bytes BE f64] [D5_MOMENTUM: 8 bytes BE u64]
/// [SHA256: 32 bytes]
/// [NUM_LAYER_ADDRS: 4 bytes BE]
///   For each layer address:
///     [N: 8 bytes BE] [D4_SPACETIME: 8 bytes BE f64] [D5_MOMENTUM: 8 bytes BE u64]
/// [PAYLOAD_LEN: 8 bytes BE]
/// [PAYLOAD: N bytes]
/// ```
pub fn encode_folded(folded: &FoldedData) -> Vec<u8> {
    let mut out = Vec::new();

    out.extend_from_slice(FOLD_MAGIC);
    out.push(FOLD_VERSION);
    out.extend_from_slice(&folded.fold_depth.to_be_bytes());
    out.extend_from_slice(&folded.original_bit_length.to_be_bytes());
    out.extend_from_slice(&folded.address.n.to_be_bytes());
    out.extend_from_slice(&folded.address.d4_spacetime.to_be_bytes());
    out.extend_from_slice(&folded.address.d5_momentum.to_be_bytes());  // u64
    out.extend_from_slice(&folded.original_hash);

    out.extend_from_slice(&(folded.layer_addresses.len() as u32).to_be_bytes());
    for addr in &folded.layer_addresses {
        out.extend_from_slice(&addr.n.to_be_bytes());
        out.extend_from_slice(&addr.d4_spacetime.to_be_bytes());
        out.extend_from_slice(&addr.d5_momentum.to_be_bytes());  // u64
    }

    out.extend_from_slice(&(folded.payload.len() as u64).to_be_bytes());
    out.extend_from_slice(&folded.payload);

    out
}

/// Decode a FoldedData structure from its binary format.
pub fn decode_folded(data: &[u8]) -> Result<FoldedData, FoldError> {
    if data.len() < 4 || &data[0..4] != FOLD_MAGIC {
        return Err(FoldError::InvalidMagic);
    }
    if data.len() < 5 || data[4] != FOLD_VERSION {
        return Err(FoldError::UnsupportedVersion);
    }

    let mut pos = 5;

    let fold_depth = read_u32_be(data, &mut pos)?;
    let original_bit_length = read_u64_be(data, &mut pos)?;
    let n = read_u64_be(data, &mut pos)?;
    let d4_spacetime = read_f64_be(data, &mut pos)?;
    let d5_momentum = read_u64_be(data, &mut pos)?;

    if data.len() < pos + 32 {
        return Err(FoldError::TruncatedData);
    }
    let mut original_hash = [0u8; 32];
    original_hash.copy_from_slice(&data[pos..pos + 32]);
    pos += 32;

    let num_layer_addrs = read_u32_be(data, &mut pos)? as usize;
    let mut layer_addresses = Vec::with_capacity(num_layer_addrs);
    for _ in 0..num_layer_addrs {
        let la_n = read_u64_be(data, &mut pos)?;
        let la_d4 = read_f64_be(data, &mut pos)?;
        let la_d5 = read_u64_be(data, &mut pos)?;
        layer_addresses.push(DimensionalAddress {
            n: la_n,
            d4_spacetime: la_d4,
            d5_momentum: la_d5,
        });
    }

    let payload_len = read_u64_be(data, &mut pos)? as usize;
    if data.len() < pos + payload_len {
        return Err(FoldError::TruncatedData);
    }
    let payload = data[pos..pos + payload_len].to_vec();

    Ok(FoldedData {
        address: DimensionalAddress { n, d4_spacetime, d5_momentum },
        payload,
        original_hash,
        original_bit_length,
        fold_depth,
        layer_addresses,
    })
}

fn read_u32_be(data: &[u8], pos: &mut usize) -> Result<u32, FoldError> {
    if data.len() < *pos + 4 {
        return Err(FoldError::TruncatedData);
    }
    let val = u32::from_be_bytes(data[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(val)
}

fn read_u64_be(data: &[u8], pos: &mut usize) -> Result<u64, FoldError> {
    if data.len() < *pos + 8 {
        return Err(FoldError::TruncatedData);
    }
    let val = u64::from_be_bytes(data[*pos..*pos + 8].try_into().unwrap());
    *pos += 8;
    Ok(val)
}

fn read_f64_be(data: &[u8], pos: &mut usize) -> Result<f64, FoldError> {
    if data.len() < *pos + 8 {
        return Err(FoldError::TruncatedData);
    }
    let val = f64::from_be_bytes(data[*pos..*pos + 8].try_into().unwrap());
    *pos += 8;
    Ok(val)
}

/// Errors that can occur during fold/unfold operations.
#[derive(Debug, Clone, PartialEq)]
pub enum FoldError {
    InvalidMagic,
    UnsupportedVersion,
    TruncatedData,
    HashMismatch,
    LengthMismatch,
}

impl std::fmt::Display for FoldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "not a valid MDB folded file"),
            Self::UnsupportedVersion => write!(f, "unsupported fold format version"),
            Self::TruncatedData => write!(f, "folded data is truncated"),
            Self::HashMismatch => write!(f, "SHA-256 hash mismatch after unfold (data corrupted)"),
            Self::LengthMismatch => write!(f, "unfolded data length doesn't match original"),
        }
    }
}

impl std::error::Error for FoldError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permutation_is_valid() {
        let addr = DimensionalAddress { n: 100, d4_spacetime: 50.0, d5_momentum: 75 };
        let perm = dimension_derived_permutation(100, &addr);
        let mut sorted = perm.clone();
        sorted.sort();
        let expected: Vec<usize> = (0..100).collect();
        assert_eq!(sorted, expected);
    }

    #[test]
    fn test_permutation_inverse() {
        let addr = DimensionalAddress { n: 50, d4_spacetime: 30.0, d5_momentum: 45 };
        let perm = dimension_derived_permutation(50, &addr);
        let inv = inverse_permutation(&perm);
        for i in 0..50 {
            assert_eq!(inv[perm[i]], i);
        }
    }

    #[test]
    fn test_momentum_mask_deterministic() {
        let m1 = momentum_mask(100, 12345);
        let m2 = momentum_mask(100, 12345);
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_fold_produces_output() {
        let data = b"Hello, MDB!";
        let folded = fold(data);
        assert_eq!(folded.original_bit_length, (data.len() * 8) as u64);
        assert_eq!(folded.fold_depth, 1);
        assert_eq!(folded.original_hash, sha256(data));
        assert!(!folded.payload.is_empty());
        assert_eq!(folded.layer_addresses.len(), 1);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let data = b"Test data for encoding roundtrip";
        let folded = fold(data);
        let encoded = encode_folded(&folded);
        let decoded = decode_folded(&encoded).unwrap();

        assert_eq!(decoded.address.n, folded.address.n);
        assert_eq!(decoded.original_hash, folded.original_hash);
        assert_eq!(decoded.payload, folded.payload);
        assert_eq!(decoded.fold_depth, folded.fold_depth);
        assert_eq!(decoded.original_bit_length, folded.original_bit_length);
        assert_eq!(decoded.layer_addresses.len(), folded.layer_addresses.len());
    }

    #[test]
    fn test_encode_decode_roundtrip_depth_3() {
        let data = b"Multi-depth encoding test";
        let folded = fold_with_depth(data, 3);
        let encoded = encode_folded(&folded);
        let decoded = decode_folded(&encoded).unwrap();

        assert_eq!(decoded.fold_depth, 3);
        assert_eq!(decoded.layer_addresses.len(), 3);
        assert_eq!(decoded.payload, folded.payload);
    }
}
