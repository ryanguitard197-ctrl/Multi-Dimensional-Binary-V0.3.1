//! # MDBFS — Multidimensional Binary Filesystem
//!
//! A FUSE filesystem that transparently stores all data in MDB dimensional form.
//!
//! ## Usage
//!
//! ```bash
//! # Mount MDBFS
//! mdbfs mount /mnt/mdb --store /var/lib/mdbfs
//!
//! # Now use it like a normal filesystem
//! echo "Hello, MDB!" > /mnt/mdb/hello.txt
//! cat /mnt/mdb/hello.txt  # → "Hello, MDB!" (transparently unfolded)
//!
//! # Check MDB metadata via xattrs
//! getfattr -n mdb.address /mnt/mdb/hello.txt
//! getfattr -n mdb.fold_depth /mnt/mdb/hello.txt
//!
//! # Unmount
//! fusermount -u /mnt/mdb
//! ```
//!
//! Under the hood, the file content on disk is in MDB folded form —
//! geometrically reorganized in dimensional coordinate space.

mod fs;
mod store;

use clap::{Parser, Subcommand};
use fuser::MountOption;
use log::info;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mdbfs")]
#[command(about = "MDBFS — A FUSE filesystem backed by MDB dimensional storage")]
#[command(version = mdb_core::VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Mount the MDB filesystem
    Mount {
        /// Mount point path
        mountpoint: PathBuf,

        /// Backing store directory (where folded data is persisted)
        #[arg(short, long, default_value = "/var/lib/mdbfs")]
        store: PathBuf,

        /// Allow other users to access the mount
        #[arg(long)]
        allow_other: bool,

        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,

        /// Default fold depth for new files
        #[arg(long, default_value = "1")]
        fold_depth: u32,

        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },

    /// Show information about the backing store
    Info {
        /// Backing store directory
        #[arg(short, long, default_value = "/var/lib/mdbfs")]
        store: PathBuf,
    },

    /// Check integrity of all stored data
    Fsck {
        /// Backing store directory
        #[arg(short, long, default_value = "/var/lib/mdbfs")]
        store: PathBuf,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mount {
            mountpoint,
            store: store_path,
            allow_other,
            foreground,
            fold_depth: _fold_depth,
            debug,
        } => {
            // Initialize logging
            if debug {
                env_logger::Builder::from_env(
                    env_logger::Env::default().default_filter_or("debug"),
                )
                .init();
            } else {
                env_logger::Builder::from_env(
                    env_logger::Env::default().default_filter_or("info"),
                )
                .init();
            }

            info!("MDBFS v{}", mdb_core::VERSION);
            info!("Store: {}", store_path.display());
            info!("Mount: {}", mountpoint.display());

            // Open the backing store
            let backing_store = match store::Store::open(&store_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error opening store at {}: {}", store_path.display(), e);
                    std::process::exit(1);
                }
            };

            let filesystem = fs::MdbFilesystem::new(backing_store);

            // Build mount options
            let mut options = vec![
                MountOption::FSName("mdbfs".to_string()),
                MountOption::AutoUnmount,
            ];

            if allow_other {
                options.push(MountOption::AllowOther);
            }

            if !foreground {
                // For now, always run in foreground
                // (daemonization requires more work)
            }

            info!("Mounting MDBFS...");

            // Ensure mount point exists
            if !mountpoint.exists() {
                if let Err(e) = std::fs::create_dir_all(&mountpoint) {
                    eprintln!(
                        "Error creating mount point {}: {}",
                        mountpoint.display(),
                        e
                    );
                    std::process::exit(1);
                }
            }

            // Mount!
            if let Err(e) = fuser::mount2(filesystem, &mountpoint, &options) {
                eprintln!("Error mounting MDBFS: {}", e);
                std::process::exit(1);
            }

            info!("MDBFS unmounted.");
        }

        Commands::Info { store: store_path } => {
            let backing_store = match store::Store::open(&store_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error opening store at {}: {}", store_path.display(), e);
                    std::process::exit(1);
                }
            };

            let stats = backing_store.statfs();
            println!("MDBFS Store Information");
            println!("=======================");
            println!("Location:     {}", store_path.display());
            println!("Total inodes: {}", stats.files);
            println!("Block size:   {} bytes", stats.bsize);

            // Count files and directories
            let mut files = 0u64;
            let mut dirs = 0u64;
            let mut symlinks = 0u64;
            let mut total_size = 0u64;

            // Walk all inodes from the store directory
            let inodes_dir = store_path.join("inodes");
            if let Ok(entries) = std::fs::read_dir(&inodes_dir) {
                for entry in entries.flatten() {
                    if let Ok(bytes) = std::fs::read(entry.path()) {
                        if let Ok(meta) =
                            serde_json::from_slice::<store::InodeMeta>(&bytes)
                        {
                            match meta.kind {
                                store::InodeKind::File => {
                                    files += 1;
                                    total_size += meta.size;
                                }
                                store::InodeKind::Directory => dirs += 1,
                                store::InodeKind::Symlink => symlinks += 1,
                            }
                        }
                    }
                }
            }

            println!("Files:        {}", files);
            println!("Directories:  {}", dirs);
            println!("Symlinks:     {}", symlinks);
            println!("Total size:   {} bytes (unfolded)", total_size);

            // Check folded size on disk
            let data_dir = store_path.join("data");
            let mut folded_size = 0u64;
            if let Ok(entries) = std::fs::read_dir(&data_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_meta) = entry.metadata() {
                        folded_size += file_meta.len();
                    }
                }
            }
            println!("Folded size:  {} bytes (on disk)", folded_size);
        }

        Commands::Fsck {
            store: store_path,
            verbose,
        } => {
            println!("Checking MDBFS integrity at {}...", store_path.display());

            let data_dir = store_path.join("data");
            let mut checked = 0u32;
            let mut errors = 0u32;

            if let Ok(entries) = std::fs::read_dir(&data_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "mdb").unwrap_or(false) {
                        checked += 1;
                        let ino_str = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("?");

                        match std::fs::read(&path) {
                            Ok(bytes) if bytes.is_empty() => {
                                if verbose {
                                    println!("  ino {}: empty (ok)", ino_str);
                                }
                            }
                            Ok(bytes) => {
                                match mdb_core::fold::decode_folded(&bytes) {
                                    Ok(folded) => {
                                        match mdb_core::unfold::unfold(&folded) {
                                            Ok(data) => {
                                                if verbose {
                                                    println!(
                                                        "  ino {}: ok ({} bytes, depth {})",
                                                        ino_str,
                                                        data.len(),
                                                        folded.fold_depth
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                errors += 1;
                                                println!(
                                                    "  ino {}: UNFOLD ERROR: {}",
                                                    ino_str, e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        errors += 1;
                                        println!("  ino {}: DECODE ERROR: {}", ino_str, e);
                                    }
                                }
                            }
                            Err(e) => {
                                errors += 1;
                                println!("  ino {}: READ ERROR: {}", ino_str, e);
                            }
                        }
                    }
                }
            }

            println!("\nChecked {} files, {} errors.", checked, errors);
            if errors > 0 {
                std::process::exit(1);
            }
        }
    }
}
