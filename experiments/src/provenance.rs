//! # Data Provenance
//!
//! Every piece of data in an MDB experiment must have a traceable origin.
//! This module tracks where data came from, when it was accessed, and
//! provides cryptographic verification via SHA-256 hashes.
//!
//! ## Why This Matters
//!
//! If MDB discovers a new drug candidate or optimal supply chain route,
//! the result is only as credible as the input data. Without provenance,
//! results are unpublishable, unreproducible, and useless.
//!
//! ## Supported Sources
//!
//! - **DOI** — Digital Object Identifier for published data
//! - **URL** — Web-accessible data with fetch timestamp
//! - **File** — Local file with SHA-256 hash
//! - **Database** — Query against a specific database version
//! - **API** — External API call with request/response hash
//! - **Computed** — Derived from other provenance-tracked data
//! - **Manual** — Human-entered data with justification

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Data provenance — where did this data come from?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    /// Source type.
    pub source_type: SourceType,
    /// Human-readable description.
    pub description: String,
    /// When this data was retrieved/created (ISO 8601).
    pub retrieved_at: String,
    /// SHA-256 hash of the raw data.
    pub data_hash: String,
    /// Parent sources (for computed/derived data).
    pub derived_from: Vec<String>,
}

/// Type of data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    /// Published data with a DOI.
    Doi {
        doi: String,
        title: Option<String>,
        authors: Option<Vec<String>>,
        year: Option<u32>,
    },
    /// Web-accessible data.
    Url {
        url: String,
        accessed_at: String,
    },
    /// Local file.
    File {
        path: String,
        size_bytes: u64,
    },
    /// Database query.
    Database {
        db_name: String,
        db_version: String,
        query: String,
    },
    /// External API.
    Api {
        endpoint: String,
        request_hash: String,
    },
    /// Computed from other data.
    Computed {
        method: String,
        parent_hashes: Vec<String>,
    },
    /// Human-entered with justification.
    Manual {
        entered_by: String,
        justification: String,
    },
}

impl DataSource {
    /// Create a DOI source.
    pub fn doi(doi: &str) -> Self {
        Self {
            source_type: SourceType::Doi {
                doi: doi.to_string(),
                title: None,
                authors: None,
                year: None,
            },
            description: format!("DOI: {}", doi),
            retrieved_at: now_iso8601(),
            data_hash: String::new(),
            derived_from: Vec::new(),
        }
    }

    /// Create a DOI source with full citation.
    pub fn doi_full(
        doi: &str,
        title: &str,
        authors: Vec<String>,
        year: u32,
    ) -> Self {
        Self {
            source_type: SourceType::Doi {
                doi: doi.to_string(),
                title: Some(title.to_string()),
                authors: Some(authors),
                year: Some(year),
            },
            description: format!("{} ({}) DOI: {}", title, year, doi),
            retrieved_at: now_iso8601(),
            data_hash: String::new(),
            derived_from: Vec::new(),
        }
    }

    /// Create a URL source.
    pub fn url(url: &str) -> Self {
        Self {
            source_type: SourceType::Url {
                url: url.to_string(),
                accessed_at: now_iso8601(),
            },
            description: format!("URL: {}", url),
            retrieved_at: now_iso8601(),
            data_hash: String::new(),
            derived_from: Vec::new(),
        }
    }

    /// Create a file source.
    pub fn file(path: &str, size_bytes: u64) -> Self {
        Self {
            source_type: SourceType::File {
                path: path.to_string(),
                size_bytes,
            },
            description: format!("File: {} ({} bytes)", path, size_bytes),
            retrieved_at: now_iso8601(),
            data_hash: String::new(),
            derived_from: Vec::new(),
        }
    }

    /// Create a database source.
    pub fn database(db_name: &str, db_version: &str, query: &str) -> Self {
        Self {
            source_type: SourceType::Database {
                db_name: db_name.to_string(),
                db_version: db_version.to_string(),
                query: query.to_string(),
            },
            description: format!("DB: {} v{}", db_name, db_version),
            retrieved_at: now_iso8601(),
            data_hash: String::new(),
            derived_from: Vec::new(),
        }
    }

    /// Create an API source.
    pub fn api(endpoint: &str, request_hash: &str) -> Self {
        Self {
            source_type: SourceType::Api {
                endpoint: endpoint.to_string(),
                request_hash: request_hash.to_string(),
            },
            description: format!("API: {}", endpoint),
            retrieved_at: now_iso8601(),
            data_hash: String::new(),
            derived_from: Vec::new(),
        }
    }

    /// Create a computed/derived source.
    pub fn computed(method: &str, parent_hashes: Vec<String>) -> Self {
        Self {
            source_type: SourceType::Computed {
                method: method.to_string(),
                parent_hashes: parent_hashes.clone(),
            },
            description: format!("Computed via {} from {} sources", method, parent_hashes.len()),
            retrieved_at: now_iso8601(),
            data_hash: String::new(),
            derived_from: parent_hashes,
        }
    }

    /// Create a manual entry source.
    pub fn manual(entered_by: &str, justification: &str) -> Self {
        Self {
            source_type: SourceType::Manual {
                entered_by: entered_by.to_string(),
                justification: justification.to_string(),
            },
            description: format!("Manual entry by {}: {}", entered_by, justification),
            retrieved_at: now_iso8601(),
            data_hash: String::new(),
            derived_from: Vec::new(),
        }
    }

    /// Compute and set the data hash from raw bytes.
    pub fn with_hash(mut self, data: &[u8]) -> Self {
        self.data_hash = sha256_hex(data);
        self
    }

    /// Verify that data matches the stored hash.
    pub fn verify(&self, data: &[u8]) -> bool {
        if self.data_hash.is_empty() {
            return false; // No hash to verify against
        }
        sha256_hex(data) == self.data_hash
    }
}

/// Compute SHA-256 hash as hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Get current time as ISO 8601 string.
fn now_iso8601() -> String {
    // Using std::time to avoid mandatory chrono dependency at this level
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", duration.as_secs())
}

/// A provenance chain — tracks all sources used in an experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceChain {
    /// All data sources, keyed by field name.
    pub sources: std::collections::HashMap<String, DataSource>,
    /// Combined hash of all input data (for the reproducibility manifest).
    pub combined_hash: String,
}

impl ProvenanceChain {
    pub fn new() -> Self {
        Self {
            sources: std::collections::HashMap::new(),
            combined_hash: String::new(),
        }
    }

    /// Add a source for a field.
    pub fn add(&mut self, field_name: &str, source: DataSource) {
        self.sources.insert(field_name.to_string(), source);
    }

    /// Compute the combined hash of all sources.
    pub fn finalize(&mut self) {
        let mut hasher = Sha256::new();
        let mut keys: Vec<&String> = self.sources.keys().collect();
        keys.sort(); // Deterministic ordering

        for key in keys {
            let source = &self.sources[key];
            hasher.update(key.as_bytes());
            hasher.update(source.data_hash.as_bytes());
        }

        self.combined_hash = format!("{:x}", hasher.finalize());
    }

    /// Verify all sources against provided data.
    pub fn verify_all(&self, data: &std::collections::HashMap<String, Vec<u8>>) -> Vec<String> {
        let mut failures = Vec::new();
        for (field, source) in &self.sources {
            if let Some(field_data) = data.get(field) {
                if !source.data_hash.is_empty() && !source.verify(field_data) {
                    failures.push(format!(
                        "Field '{}': hash mismatch (data may have been modified)",
                        field
                    ));
                }
            }
        }
        failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let hash = sha256_hex(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_data_source_verify() {
        let source = DataSource::doi("10.1234/test").with_hash(b"my data");
        assert!(source.verify(b"my data"));
        assert!(!source.verify(b"different data"));
    }

    #[test]
    fn test_provenance_chain() {
        let mut chain = ProvenanceChain::new();
        chain.add("field_a", DataSource::doi("10.1234/a").with_hash(b"data_a"));
        chain.add("field_b", DataSource::url("https://example.com").with_hash(b"data_b"));
        chain.finalize();
        assert!(!chain.combined_hash.is_empty());
    }
}
