//! # Backing Store
//!
//! Persistent storage layer for MDBFS. All file content is stored in MDB
//! folded form on disk. The store manages:
//!
//! - Inode metadata (type, permissions, timestamps, size)
//! - Directory entries (name → inode mappings)
//! - File content (folded MDB data)
//! - Inode allocation
//!
//! The backing store directory layout:
//! ```text
//! <store_root>/
//! ├── meta.json          # Filesystem metadata (next inode, etc.)
//! ├── inodes/            # Per-inode metadata files
//! │   ├── 1.json         # Root directory inode
//! │   ├── 2.json         # ...
//! │   └── ...
//! └── data/              # Folded file content
//!     ├── 2.mdb          # Folded data for inode 2
//!     └── ...
//! ```

use mdb_core::fold::{encode_folded, fold_with_depth, FoldError};
use mdb_core::unfold::unfold;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Default fold depth for new files.
pub const DEFAULT_FOLD_DEPTH: u32 = 1;

/// Root inode number (always 1, matching FUSE convention).
pub const ROOT_INO: u64 = 1;

/// Inode types supported by MDBFS.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InodeKind {
    File,
    Directory,
    Symlink,
}

/// Metadata for a single inode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InodeMeta {
    pub ino: u64,
    pub kind: InodeKind,
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub nlink: u32,
    pub atime: (i64, u32), // (secs, nsecs) since epoch
    pub mtime: (i64, u32),
    pub ctime: (i64, u32),
    pub crtime: (i64, u32),
    /// For directories: name → child inode
    #[serde(default)]
    pub children: HashMap<String, u64>,
    /// For symlinks: target path
    #[serde(default)]
    pub symlink_target: Option<String>,
    /// Fold depth used for this file's content
    #[serde(default = "default_fold_depth")]
    pub fold_depth: u32,
    /// Extended attributes
    #[serde(default)]
    pub xattrs: HashMap<String, Vec<u8>>,
}

fn default_fold_depth() -> u32 {
    DEFAULT_FOLD_DEPTH
}

/// Filesystem-level metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsMeta {
    pub next_ino: u64,
    pub block_size: u32,
    pub total_inodes: u64,
}

/// The persistent backing store for MDBFS.
pub struct Store {
    root: PathBuf,
    fs_meta: FsMeta,
    /// In-memory cache of inode metadata (persisted on flush).
    inodes: HashMap<u64, InodeMeta>,
    /// In-memory cache of file content (unfolded).
    /// Only loaded on demand, evicted when memory is tight.
    content_cache: HashMap<u64, Vec<u8>>,
    /// Dirty inodes that need to be flushed.
    dirty_inodes: std::collections::HashSet<u64>,
    /// Dirty content that needs to be folded and written.
    dirty_content: std::collections::HashSet<u64>,
}

impl Store {
    /// Open or create a backing store at the given path.
    pub fn open(root: &Path) -> std::io::Result<Self> {
        let inodes_dir = root.join("inodes");
        let data_dir = root.join("data");
        std::fs::create_dir_all(&inodes_dir)?;
        std::fs::create_dir_all(&data_dir)?;

        let meta_path = root.join("meta.json");
        let (fs_meta, mut inodes) = if meta_path.exists() {
            // Load existing store
            let meta_bytes = std::fs::read(&meta_path)?;
            let fs_meta: FsMeta = serde_json::from_slice(&meta_bytes).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;

            let mut inodes = HashMap::new();
            for entry in std::fs::read_dir(&inodes_dir)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(ino_str) = name.strip_suffix(".json") {
                        if let Ok(ino) = ino_str.parse::<u64>() {
                            let bytes = std::fs::read(entry.path())?;
                            let meta: InodeMeta =
                                serde_json::from_slice(&bytes).map_err(|e| {
                                    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                                })?;
                            inodes.insert(ino, meta);
                        }
                    }
                }
            }

            (fs_meta, inodes)
        } else {
            // Create fresh filesystem
            let fs_meta = FsMeta {
                next_ino: 2,
                block_size: 4096,
                total_inodes: 1,
            };
            let inodes = HashMap::new();
            (fs_meta, inodes)
        };

        // Ensure root inode exists
        if !inodes.contains_key(&ROOT_INO) {
            let now = system_time_to_pair(SystemTime::now());
            let root_inode = InodeMeta {
                ino: ROOT_INO,
                kind: InodeKind::Directory,
                size: 0,
                mode: 0o755,
                uid: unsafe { libc::getuid() },
                gid: unsafe { libc::getgid() },
                nlink: 2,
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                children: HashMap::new(),
                symlink_target: None,
                fold_depth: DEFAULT_FOLD_DEPTH,
                xattrs: HashMap::new(),
            };
            inodes.insert(ROOT_INO, root_inode);
        }

        let mut store = Store {
            root: root.to_path_buf(),
            fs_meta,
            inodes,
            content_cache: HashMap::new(),
            dirty_inodes: std::collections::HashSet::new(),
            dirty_content: std::collections::HashSet::new(),
        };

        // Mark root as dirty so it gets persisted
        store.dirty_inodes.insert(ROOT_INO);
        store.flush()?;

        Ok(store)
    }

    /// Allocate a new inode number.
    pub fn alloc_ino(&mut self) -> u64 {
        let ino = self.fs_meta.next_ino;
        self.fs_meta.next_ino += 1;
        self.fs_meta.total_inodes += 1;
        ino
    }

    /// Get inode metadata (immutable).
    pub fn get_inode(&self, ino: u64) -> Option<&InodeMeta> {
        self.inodes.get(&ino)
    }

    /// Get inode metadata (mutable) and mark as dirty.
    pub fn get_inode_mut(&mut self, ino: u64) -> Option<&mut InodeMeta> {
        if self.inodes.contains_key(&ino) {
            self.dirty_inodes.insert(ino);
            self.inodes.get_mut(&ino)
        } else {
            None
        }
    }

    /// Insert a new inode and mark it dirty.
    pub fn insert_inode(&mut self, meta: InodeMeta) {
        let ino = meta.ino;
        self.inodes.insert(ino, meta);
        self.dirty_inodes.insert(ino);
    }

    /// Remove an inode entirely.
    pub fn remove_inode(&mut self, ino: u64) {
        self.inodes.remove(&ino);
        self.content_cache.remove(&ino);
        self.dirty_inodes.remove(&ino);
        self.dirty_content.remove(&ino);

        // Remove persisted files
        let _ = std::fs::remove_file(self.root.join(format!("inodes/{}.json", ino)));
        let _ = std::fs::remove_file(self.root.join(format!("data/{}.mdb", ino)));

        self.fs_meta.total_inodes = self.fs_meta.total_inodes.saturating_sub(1);
    }

    /// Read file content (unfolds from disk if not cached).
    pub fn read_content(&mut self, ino: u64) -> Result<Vec<u8>, StoreError> {
        // Check cache first
        if let Some(data) = self.content_cache.get(&ino) {
            return Ok(data.clone());
        }

        // Load from disk — data is stored folded
        let data_path = self.root.join(format!("data/{}.mdb", ino));
        if !data_path.exists() {
            // File has no content yet (empty file)
            return Ok(Vec::new());
        }

        let folded_bytes = std::fs::read(&data_path)?;
        if folded_bytes.is_empty() {
            return Ok(Vec::new());
        }

        let folded = mdb_core::fold::decode_folded(&folded_bytes)
            .map_err(|e| StoreError::FoldError(e))?;
        let data = unfold(&folded).map_err(|e| StoreError::FoldError(e))?;

        // Cache the unfolded data
        self.content_cache.insert(ino, data.clone());

        Ok(data)
    }

    /// Write file content (will be folded on flush).
    pub fn write_content(&mut self, ino: u64, data: Vec<u8>) {
        // Update size in metadata
        if let Some(meta) = self.inodes.get_mut(&ino) {
            meta.size = data.len() as u64;
            let now = system_time_to_pair(SystemTime::now());
            meta.mtime = now;
            meta.ctime = now;
            self.dirty_inodes.insert(ino);
        }

        self.content_cache.insert(ino, data);
        self.dirty_content.insert(ino);
    }

    /// Write a range of bytes into file content.
    pub fn write_content_range(
        &mut self,
        ino: u64,
        offset: u64,
        data: &[u8],
    ) -> Result<(), StoreError> {
        let mut content = self.read_content(ino)?;
        let end = offset as usize + data.len();

        // Extend if needed
        if end > content.len() {
            content.resize(end, 0);
        }

        content[offset as usize..end].copy_from_slice(data);
        self.write_content(ino, content);
        Ok(())
    }

    /// Truncate file content to a given size.
    pub fn truncate_content(&mut self, ino: u64, size: u64) -> Result<(), StoreError> {
        let mut content = self.read_content(ino)?;
        content.resize(size as usize, 0);
        self.write_content(ino, content);
        Ok(())
    }

    /// Flush all dirty data to disk.
    pub fn flush(&mut self) -> std::io::Result<()> {
        // Flush dirty inodes
        for &ino in &self.dirty_inodes {
            if let Some(meta) = self.inodes.get(&ino) {
                let path = self.root.join(format!("inodes/{}.json", ino));
                let bytes = serde_json::to_vec_pretty(meta).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::Other, e)
                })?;
                std::fs::write(&path, &bytes)?;
            }
        }
        self.dirty_inodes.clear();

        // Flush dirty content — fold and write
        for &ino in &self.dirty_content {
            if let Some(content) = self.content_cache.get(&ino) {
                let fold_depth = self
                    .inodes
                    .get(&ino)
                    .map(|m| m.fold_depth)
                    .unwrap_or(DEFAULT_FOLD_DEPTH);

                let data_path = self.root.join(format!("data/{}.mdb", ino));

                if content.is_empty() {
                    // Don't fold empty files — just write empty
                    std::fs::write(&data_path, b"")?;
                } else {
                    let folded = fold_with_depth(content, fold_depth);
                    let encoded = encode_folded(&folded);
                    std::fs::write(&data_path, &encoded)?;
                }
            }
        }
        self.dirty_content.clear();

        // Flush filesystem metadata
        let meta_path = self.root.join("meta.json");
        let bytes = serde_json::to_vec_pretty(&self.fs_meta).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;
        std::fs::write(&meta_path, &bytes)?;

        Ok(())
    }

    /// Lookup a child by name in a directory inode.
    pub fn lookup_child(&self, parent_ino: u64, name: &str) -> Option<u64> {
        self.inodes
            .get(&parent_ino)
            .and_then(|meta| meta.children.get(name).copied())
    }

    /// Get filesystem statistics.
    pub fn statfs(&self) -> FsStats {
        let total_data_size: u64 = self
            .inodes
            .values()
            .filter(|m| m.kind == InodeKind::File)
            .map(|m| m.size)
            .sum();

        FsStats {
            blocks: (total_data_size / self.fs_meta.block_size as u64) + 1024,
            bfree: 1024 * 1024,
            bavail: 1024 * 1024,
            files: self.fs_meta.total_inodes,
            ffree: u64::MAX - self.fs_meta.total_inodes,
            bsize: self.fs_meta.block_size,
            namelen: 255,
        }
    }
}

/// Filesystem statistics.
pub struct FsStats {
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub files: u64,
    pub ffree: u64,
    pub bsize: u32,
    pub namelen: u32,
}

/// Errors from the store layer.
#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    FoldError(FoldError),
}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "IO error: {}", e),
            StoreError::FoldError(e) => write!(f, "MDB fold error: {}", e),
        }
    }
}

impl std::error::Error for StoreError {}

/// Convert SystemTime to (secs, nsecs) pair.
pub fn system_time_to_pair(t: SystemTime) -> (i64, u32) {
    match t.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i64, d.subsec_nanos()),
        Err(_) => (0, 0),
    }
}

/// Convert (secs, nsecs) pair to SystemTime.
pub fn pair_to_system_time(pair: (i64, u32)) -> SystemTime {
    if pair.0 >= 0 {
        SystemTime::UNIX_EPOCH + std::time::Duration::new(pair.0 as u64, pair.1)
    } else {
        SystemTime::UNIX_EPOCH
    }
}
