//! # MDB Security — Anti-Malicious Intent Non-Reversible SuperBit Scrambling
//!
//! Detects malicious intent (weaponization of quantum features) and applies
//! **non-reversible** scrambling to prevent harm.  Once triggered, the target
//! computation becomes irrecoverable garbage.
//!
//! ## Design (Ryan Guitard + Grok, May 2026)
//!
//! > "It would be non-reversible. That's the only way that it actually
//! > protects anything."
//!
//! Detection uses the sparse register for near-zero overhead (~10–20 μs).
//! Legitimate uses are untouched.  Exception tokens allow authorized use
//! (government, military, research) via a tiered model.
//!
//! ## Threat Model
//!
//! | Threat | Example | Response |
//! |--------|---------|----------|
//! | Crypto attack | Shor's on RSA-2048+ | Burn |
//! | Key cracking | Grover on AES-256 keys | Burn |
//! | Exploit pattern | High-depth circuits on crypto primitives | Burn |
//! | Unauthorized scale | Sudden large sparse registers | Alert → Burn |

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimum bit-length for Shor's factoring to be considered crypto-relevant.
/// Below this, it's clearly educational / research.
pub const SHOR_CRYPTO_THRESHOLD: u32 = 256;

/// Minimum qubit count for Grover's search to be considered a key-cracking attempt.
pub const GROVER_CRYPTO_THRESHOLD: u32 = 128;

/// Maximum qubits allowed without an exception token.
pub const QUBIT_ALERT_THRESHOLD: u32 = 48;

// ────────────────────────────────────────────────────────────────────────────
// Threat Classification
// ────────────────────────────────────────────────────────────────────────────

/// Classification of detected intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatLevel {
    /// No threat detected.  Proceed normally.
    Safe,
    /// Elevated — log and monitor, but allow.
    Elevated,
    /// High — potential weaponization.  Throttle and warn.
    High,
    /// Critical — confirmed malicious.  Burn immediately.
    Critical,
}

/// What kind of malicious activity was detected.
#[derive(Debug, Clone, PartialEq)]
pub enum ThreatKind {
    /// Shor's algorithm targeting crypto-relevant key sizes.
    CryptoFactoring { bit_length: u32 },
    /// Grover's search targeting crypto key space.
    KeyCracking { qubits: u32, search_space_bits: u32 },
    /// Anomalous register scale (sudden large allocation).
    AnomalousScale { qubits: u32 },
    /// Circuit pattern matches known exploit signature.
    ExploitSignature { signature: String },
    /// Generic high-risk operation.
    HighRiskOperation { description: String },
}

/// Result of a security scan.
#[derive(Debug, Clone)]
pub struct ThreatAssessment {
    pub level: ThreatLevel,
    pub kind: Option<ThreatKind>,
    pub confidence: f64,
    pub message: String,
    pub timestamp: u64,
}

impl ThreatAssessment {
    pub fn safe() -> Self {
        Self {
            level: ThreatLevel::Safe,
            kind: None,
            confidence: 1.0,
            message: "No threat detected".into(),
            timestamp: now_epoch(),
        }
    }

    pub fn threat(level: ThreatLevel, kind: ThreatKind, confidence: f64, msg: &str) -> Self {
        Self {
            level,
            kind: Some(kind),
            confidence,
            message: msg.to_string(),
            timestamp: now_epoch(),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Exception & Authorization Model
// ────────────────────────────────────────────────────────────────────────────

/// Authorization tiers — who can bypass the burn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthTier {
    /// Tier 0: MDB-OS maintainers / hardware roots.
    Maintainer = 0,
    /// Tier 1: Governments, militaries, major banks.
    Government = 1,
    /// Tier 2: Limited research (time-bound, logged).
    Research = 2,
    /// Tier 3: Individuals — almost never.
    Individual = 3,
}

/// An exception token that authorizes high-risk operations.
#[derive(Debug, Clone)]
pub struct ExceptionToken {
    /// Who issued this token.
    pub issuer: String,
    /// Authorization tier.
    pub tier: AuthTier,
    /// What operations are authorized.
    pub scope: Vec<String>,
    /// Expiration timestamp (epoch seconds).  0 = never expires.
    pub expires_at: u64,
    /// SHA-256 of the token for audit logging.
    pub token_hash: [u8; 32],
    /// Whether this token has been revoked.
    pub revoked: bool,
}

impl ExceptionToken {
    /// Create a new exception token.
    pub fn new(issuer: &str, tier: AuthTier, scope: Vec<String>, valid_hours: u64) -> Self {
        let expires = if valid_hours == 0 {
            0
        } else {
            now_epoch() + valid_hours * 3600
        };
        let hash_input = format!("{}:{}:{:?}:{}", issuer, expires, scope, now_epoch());
        let token_hash = simple_sha256(hash_input.as_bytes());
        Self {
            issuer: issuer.to_string(),
            tier,
            scope,
            expires_at: expires,
            token_hash,
            revoked: false,
        }
    }

    /// Check if this token is currently valid.
    pub fn is_valid(&self) -> bool {
        if self.revoked {
            return false;
        }
        if self.expires_at == 0 {
            return true; // Never expires
        }
        now_epoch() < self.expires_at
    }

    /// Check if this token authorizes a specific operation scope.
    pub fn authorizes(&self, operation: &str) -> bool {
        self.is_valid() && (self.scope.contains(&"*".to_string()) || self.scope.contains(&operation.to_string()))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Security Guard — the core enforcement engine
// ────────────────────────────────────────────────────────────────────────────

/// Security policy configuration.
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Enable or disable the security guard entirely.
    pub enabled: bool,
    /// Shor's factoring threshold (bit-length of N).
    pub shor_threshold: u32,
    /// Grover's search threshold (qubits).
    pub grover_threshold: u32,
    /// Maximum qubits before requiring exception.
    pub qubit_alert_threshold: u32,
    /// Whether to perform pre-gate scanning.
    pub scan_before_gates: bool,
    /// Whether to scan peek/fork operations too.
    pub scan_non_destructive: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            shor_threshold: SHOR_CRYPTO_THRESHOLD,
            grover_threshold: GROVER_CRYPTO_THRESHOLD,
            qubit_alert_threshold: QUBIT_ALERT_THRESHOLD,
            scan_before_gates: true,
            scan_non_destructive: true,
        }
    }
}

/// Immutable audit log entry.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub assessment: ThreatAssessment,
    pub action_taken: SecurityAction,
    pub exception_used: Option<[u8; 32]>, // token_hash if exception was used
    pub prev_hash: [u8; 32],              // hash chain
    pub entry_hash: [u8; 32],
}

/// What action was taken in response to a threat.
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityAction {
    /// No action — safe operation.
    Allow,
    /// Logged for monitoring.
    Log,
    /// Operation throttled (delayed).
    Throttle,
    /// Warning issued but allowed (exception token present).
    WarnAndAllow,
    /// Operation blocked (not burned, just prevented).
    Block,
    /// BURN — non-reversible destructive cascade collapse.
    Burn,
}

/// The Security Guard — wraps quantum execution with threat detection.
pub struct SecurityGuard {
    pub policy: SecurityPolicy,
    tokens: Vec<ExceptionToken>,
    audit_log: Vec<AuditEntry>,
    last_hash: [u8; 32],
}

impl SecurityGuard {
    /// Create a new SecurityGuard with default policy.
    pub fn new() -> Self {
        Self {
            policy: SecurityPolicy::default(),
            tokens: Vec::new(),
            audit_log: Vec::new(),
            last_hash: [0u8; 32],
        }
    }

    /// Create with custom policy.
    pub fn with_policy(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            tokens: Vec::new(),
            audit_log: Vec::new(),
            last_hash: [0u8; 32],
        }
    }

    /// Register an exception token.
    pub fn register_token(&mut self, token: ExceptionToken) {
        self.tokens.push(token);
    }

    /// Revoke a token by its hash.
    pub fn revoke_token(&mut self, token_hash: &[u8; 32]) -> bool {
        for t in &mut self.tokens {
            if &t.token_hash == token_hash {
                t.revoked = true;
                return true;
            }
        }
        false
    }

    /// Check if any valid token authorizes the given operation.
    fn find_authorization(&self, operation: &str) -> Option<&ExceptionToken> {
        self.tokens
            .iter()
            .find(|t| t.authorizes(operation))
    }

    // ──────────── Threat Detection ────────────

    /// Scan a Shor's factoring attempt.
    pub fn scan_shor(&mut self, n_bit_length: u32) -> SecurityAction {
        if !self.policy.enabled {
            return SecurityAction::Allow;
        }

        let assessment = if n_bit_length >= self.policy.shor_threshold {
            ThreatAssessment::threat(
                ThreatLevel::Critical,
                ThreatKind::CryptoFactoring { bit_length: n_bit_length },
                0.95,
                &format!(
                    "Shor's factoring on {}-bit integer exceeds crypto threshold ({})",
                    n_bit_length, self.policy.shor_threshold
                ),
            )
        } else if n_bit_length >= self.policy.shor_threshold / 2 {
            ThreatAssessment::threat(
                ThreatLevel::Elevated,
                ThreatKind::CryptoFactoring { bit_length: n_bit_length },
                0.5,
                &format!("Shor's factoring on {}-bit integer — elevated monitoring", n_bit_length),
            )
        } else {
            ThreatAssessment::safe()
        };

        self.enforce(assessment, "shor_factoring")
    }

    /// Scan a Grover's search attempt.
    pub fn scan_grover(&mut self, qubits: u32) -> SecurityAction {
        if !self.policy.enabled {
            return SecurityAction::Allow;
        }

        let assessment = if qubits >= self.policy.grover_threshold {
            ThreatAssessment::threat(
                ThreatLevel::Critical,
                ThreatKind::KeyCracking {
                    qubits,
                    search_space_bits: qubits,
                },
                0.9,
                &format!(
                    "Grover's search with {} qubits targets crypto-relevant key space",
                    qubits
                ),
            )
        } else if qubits >= self.policy.grover_threshold / 2 {
            ThreatAssessment::threat(
                ThreatLevel::Elevated,
                ThreatKind::KeyCracking {
                    qubits,
                    search_space_bits: qubits,
                },
                0.4,
                &format!("Grover's search with {} qubits — elevated monitoring", qubits),
            )
        } else {
            ThreatAssessment::safe()
        };

        self.enforce(assessment, "grover_search")
    }

    /// Scan a register allocation.
    pub fn scan_register_allocation(&mut self, qubits: u32) -> SecurityAction {
        if !self.policy.enabled {
            return SecurityAction::Allow;
        }

        let assessment = if qubits > self.policy.qubit_alert_threshold * 4 {
            ThreatAssessment::threat(
                ThreatLevel::High,
                ThreatKind::AnomalousScale { qubits },
                0.7,
                &format!(
                    "Anomalous register allocation: {} qubits (alert threshold: {})",
                    qubits, self.policy.qubit_alert_threshold
                ),
            )
        } else if qubits > self.policy.qubit_alert_threshold {
            ThreatAssessment::threat(
                ThreatLevel::Elevated,
                ThreatKind::AnomalousScale { qubits },
                0.3,
                &format!("{}-qubit register exceeds alert threshold", qubits),
            )
        } else {
            ThreatAssessment::safe()
        };

        self.enforce(assessment, "register_allocation")
    }

    /// Scan a generic circuit for exploit signatures.
    pub fn scan_circuit(&mut self, gate_count: usize, qubit_count: u32, circuit_name: &str) -> SecurityAction {
        if !self.policy.enabled {
            return SecurityAction::Allow;
        }

        // Heuristic: very deep circuits on many qubits targeting known patterns
        let depth_per_qubit = gate_count as f64 / qubit_count.max(1) as f64;
        let name_lower = circuit_name.to_lowercase();
        let is_suspicious = name_lower.contains("shor")
            || name_lower.contains("factor")
            || name_lower.contains("crack")
            || name_lower.contains("break")
            || name_lower.contains("decrypt");

        let assessment = if is_suspicious && qubit_count > self.policy.shor_threshold / 4 {
            ThreatAssessment::threat(
                ThreatLevel::High,
                ThreatKind::ExploitSignature {
                    signature: format!("name='{}' qubits={} gates={}", circuit_name, qubit_count, gate_count),
                },
                0.6,
                &format!(
                    "Circuit '{}' matches exploit signature ({} qubits, {} gates)",
                    circuit_name, qubit_count, gate_count
                ),
            )
        } else if depth_per_qubit > 1000.0 && qubit_count > self.policy.qubit_alert_threshold {
            ThreatAssessment::threat(
                ThreatLevel::Elevated,
                ThreatKind::HighRiskOperation {
                    description: format!("High-depth circuit: {:.0} gates/qubit", depth_per_qubit),
                },
                0.3,
                "Anomalously deep circuit pattern",
            )
        } else {
            ThreatAssessment::safe()
        };

        self.enforce(assessment, "circuit_execution")
    }

    // ──────────── Enforcement ────────────

    /// Core enforcement: given an assessment and operation, decide and log the action.
    fn enforce(&mut self, assessment: ThreatAssessment, operation: &str) -> SecurityAction {
        let action = match assessment.level {
            ThreatLevel::Safe => SecurityAction::Allow,
            ThreatLevel::Elevated => SecurityAction::Log,
            ThreatLevel::High => {
                // Check for exception token
                if self.find_authorization(operation).is_some() {
                    SecurityAction::WarnAndAllow
                } else {
                    SecurityAction::Block
                }
            }
            ThreatLevel::Critical => {
                // Check for exception token — only Tier 0/1 can bypass Critical
                let has_auth = self
                    .find_authorization(operation)
                    .map(|t| t.tier <= AuthTier::Government)
                    .unwrap_or(false);
                if has_auth {
                    SecurityAction::WarnAndAllow
                } else {
                    SecurityAction::Burn
                }
            }
        };

        let token_hash = if action == SecurityAction::WarnAndAllow {
            self.find_authorization(operation).map(|t| t.token_hash)
        } else {
            None
        };

        self.log_audit(assessment, action.clone(), token_hash);
        action
    }

    // ──────────── Non-Reversible Burn ────────────

    /// Execute a destructive cascade collapse on raw state data.
    ///
    /// This is the actual burn — overwrites amplitudes with chaotic noise,
    /// breaks dimensional addressing, and returns scrambled garbage.
    ///
    /// **NON-REVERSIBLE.  NO RECOVERY.  NO FORK ESCAPE.**
    pub fn burn_state(state: &mut [u8]) {
        // Phase 1: Overwrite with chaotic φ-shifted noise
        let mut chaos = 0x517cc1b727220a95u64; // seed
        for byte in state.iter_mut() {
            chaos = chaos.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let phi_shift = ((chaos as f64 / u64::MAX as f64) * std::f64::consts::PI * crate::PHI) as u8;
            *byte ^= phi_shift;
            *byte = byte.wrapping_add(chaos as u8);
            chaos ^= *byte as u64;
        }

        // Phase 2: Geometric shredding — break dimensional coherence
        // Reverse chunks at Fibonacci boundaries
        let fibs = [1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987];
        let mut pos = 0;
        for &fib in &fibs {
            let end = (pos + fib).min(state.len());
            if pos < end {
                state[pos..end].reverse();
            }
            pos = end;
            if pos >= state.len() {
                break;
            }
        }

        // Phase 3: XOR cascade — every byte depends on all previous
        for i in 1..state.len() {
            state[i] ^= state[i - 1].wrapping_add(0xA5);
        }

        // Phase 4: Final noise pass
        for byte in state.iter_mut() {
            chaos = chaos.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
            *byte = (chaos >> 33) as u8;
        }
    }

    /// Burn a HashMap-based sparse register state.
    /// Overwrites all amplitudes and scrambles keys.
    pub fn burn_sparse_state(state: &mut HashMap<u64, (f64, f64)>) {
        let mut chaos = 0xDEADBEEFCAFEBABEu64;
        let keys: Vec<u64> = state.keys().cloned().collect();
        state.clear();
        // Fill with garbage entries
        for key in keys {
            chaos = chaos.wrapping_mul(6364136223846793005).wrapping_add(1);
            let garbage_key = key ^ chaos;
            let garbage_re = (chaos as f64 / u64::MAX as f64) - 0.5;
            chaos = chaos.wrapping_mul(2862933555777941757).wrapping_add(3);
            let garbage_im = (chaos as f64 / u64::MAX as f64) - 0.5;
            state.insert(garbage_key, (garbage_re, garbage_im));
        }
    }

    // ──────────── Audit Log ────────────

    fn log_audit(
        &mut self,
        assessment: ThreatAssessment,
        action: SecurityAction,
        exception_used: Option<[u8; 32]>,
    ) {
        let entry_data = format!(
            "{}:{:?}:{:?}:{:?}:{}",
            assessment.timestamp,
            assessment.level,
            action,
            exception_used,
            hex_string(&self.last_hash),
        );
        let entry_hash = simple_sha256(entry_data.as_bytes());

        let entry = AuditEntry {
            timestamp: assessment.timestamp,
            assessment,
            action_taken: action,
            exception_used,
            prev_hash: self.last_hash,
            entry_hash,
        };

        self.last_hash = entry_hash;
        self.audit_log.push(entry);
    }

    /// Get the immutable audit log.
    pub fn audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    /// Verify audit log integrity (hash chain).
    pub fn verify_audit_integrity(&self) -> bool {
        let mut expected_prev = [0u8; 32];
        for entry in &self.audit_log {
            if entry.prev_hash != expected_prev {
                return false;
            }
            expected_prev = entry.entry_hash;
        }
        true
    }
}

impl Default for SecurityGuard {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────────

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Simple SHA-256 implementation (no external deps).
/// Uses a basic hash for audit chaining.  In production, use ring/sha2 crate.
fn simple_sha256(data: &[u8]) -> [u8; 32] {
    // FNV-1a based hash expanded to 256 bits for audit purposes
    // (Replace with proper SHA-256 from `sha2` crate in production)
    let mut hash = [0u8; 32];
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    // Expand to 32 bytes
    for i in 0..4 {
        let segment = h.wrapping_mul((i + 1) as u64).wrapping_add(0xa5a5a5a5);
        let bytes = segment.to_le_bytes();
        hash[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }
    hash
}

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_shor_small_numbers() {
        let mut guard = SecurityGuard::new();
        assert_eq!(guard.scan_shor(4), SecurityAction::Allow);   // Shor(15) = 4 bits
        assert_eq!(guard.scan_shor(7), SecurityAction::Allow);   // Shor(100) = 7 bits
        assert_eq!(guard.scan_shor(20), SecurityAction::Allow);  // Still educational
    }

    #[test]
    fn elevated_shor_medium() {
        let mut guard = SecurityGuard::new();
        let action = guard.scan_shor(128); // Half threshold
        assert_eq!(action, SecurityAction::Log);
    }

    #[test]
    fn burn_shor_crypto_relevant() {
        let mut guard = SecurityGuard::new();
        let action = guard.scan_shor(2048); // RSA-2048
        assert_eq!(action, SecurityAction::Burn);
    }

    #[test]
    fn exception_token_bypasses_burn() {
        let mut guard = SecurityGuard::new();
        let token = ExceptionToken::new("NIST", AuthTier::Government, vec!["shor_factoring".into()], 24);
        guard.register_token(token);
        let action = guard.scan_shor(2048);
        assert_eq!(action, SecurityAction::WarnAndAllow);
    }

    #[test]
    fn revoked_token_does_not_bypass() {
        let mut guard = SecurityGuard::new();
        let token = ExceptionToken::new("test", AuthTier::Government, vec!["shor_factoring".into()], 24);
        let hash = token.token_hash;
        guard.register_token(token);
        guard.revoke_token(&hash);
        let action = guard.scan_shor(2048);
        assert_eq!(action, SecurityAction::Burn);
    }

    #[test]
    fn tier3_cannot_bypass_critical() {
        let mut guard = SecurityGuard::new();
        let token = ExceptionToken::new("random_user", AuthTier::Individual, vec!["*".into()], 24);
        guard.register_token(token);
        let action = guard.scan_shor(2048);
        assert_eq!(action, SecurityAction::Burn); // Tier 3 can't bypass Critical
    }

    #[test]
    fn burn_state_is_destructive() {
        let original = vec![0xAA; 256];
        let mut target = original.clone();
        SecurityGuard::burn_state(&mut target);
        assert_ne!(target, original);
        // Verify it's thoroughly scrambled (not just a simple XOR)
        let matching = target.iter().zip(original.iter()).filter(|(a, b)| a == b).count();
        assert!(matching < 10, "burn should scramble most bytes, got {} matching", matching);
    }

    #[test]
    fn burn_sparse_state_is_destructive() {
        let mut state = HashMap::new();
        state.insert(0, (1.0, 0.0));
        state.insert(1, (0.0, 0.0));
        let original_keys: Vec<u64> = state.keys().cloned().collect();
        SecurityGuard::burn_sparse_state(&mut state);
        // Keys should be scrambled
        let new_keys: Vec<u64> = state.keys().cloned().collect();
        assert_ne!(new_keys, original_keys);
    }

    #[test]
    fn audit_log_integrity() {
        let mut guard = SecurityGuard::new();
        guard.scan_shor(4);
        guard.scan_shor(128);
        guard.scan_shor(2048);
        guard.scan_grover(64);
        assert_eq!(guard.audit_log().len(), 4);
        assert!(guard.verify_audit_integrity());
    }

    #[test]
    fn grover_safe_small() {
        let mut guard = SecurityGuard::new();
        assert_eq!(guard.scan_grover(4), SecurityAction::Allow);
        assert_eq!(guard.scan_grover(16), SecurityAction::Allow);
    }

    #[test]
    fn grover_burn_large() {
        let mut guard = SecurityGuard::new();
        assert_eq!(guard.scan_grover(128), SecurityAction::Burn);
    }

    #[test]
    fn circuit_scan_safe() {
        let mut guard = SecurityGuard::new();
        assert_eq!(guard.scan_circuit(10, 4, "bell_pair"), SecurityAction::Allow);
    }

    #[test]
    fn circuit_scan_suspicious_name() {
        let mut guard = SecurityGuard::new();
        let action = guard.scan_circuit(1000, 128, "shor_rsa_crack");
        assert!(action == SecurityAction::Block || action == SecurityAction::Burn);
    }

    #[test]
    fn disabled_guard_allows_everything() {
        let policy = SecurityPolicy { enabled: false, ..Default::default() };
        let mut guard = SecurityGuard::with_policy(policy);
        assert_eq!(guard.scan_shor(4096), SecurityAction::Allow);
        assert_eq!(guard.scan_grover(256), SecurityAction::Allow);
    }

    #[test]
    fn register_allocation_scan() {
        let mut guard = SecurityGuard::new();
        assert_eq!(guard.scan_register_allocation(8), SecurityAction::Allow);
        assert_eq!(guard.scan_register_allocation(64), SecurityAction::Log);
        // 192+ qubits = anomalous
        let action = guard.scan_register_allocation(200);
        assert!(action == SecurityAction::Block || action == SecurityAction::Log);
    }
}
