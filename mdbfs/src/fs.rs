//! # MDBFS FUSE Filesystem Implementation
//!
//! Implements the `fuser::Filesystem` trait to expose MDB dimensional storage
//! as a standard mountable filesystem.
//!
//! Every file written through MDBFS is transparently folded into MDB
//! dimensional representation. Every file read is transparently unfolded.
//! To the user, it looks and feels like a normal filesystem — but under
//! the hood, all data lives in MDB coordinate space.

use crate::store::{
    pair_to_system_time, system_time_to_pair, InodeKind, InodeMeta, Store, ROOT_INO,
    DEFAULT_FOLD_DEPTH,
};
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr,
    Request, TimeOrNow,
};
use libc::{
    EEXIST, EINVAL, EIO, EISDIR, ENODATA, ENOENT, ENOSYS, ENOTDIR, ENOTEMPTY,
    ERANGE,
};
use log::{debug, error, warn};
use std::ffi::OsStr;
use std::time::{Duration, SystemTime};

/// TTL for cached attributes (1 second).
const TTL: Duration = Duration::from_secs(1);

/// The MDBFS filesystem.
pub struct MdbFilesystem {
    store: Store,
    /// Open file handles: fh → (ino, flags)
    open_files: std::collections::HashMap<u64, (u64, i32)>,
    /// Open dir handles: fh → ino
    open_dirs: std::collections::HashMap<u64, u64>,
    /// Next file handle.
    next_fh: u64,
}

impl MdbFilesystem {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            open_files: std::collections::HashMap::new(),
            open_dirs: std::collections::HashMap::new(),
            next_fh: 1,
        }
    }

    fn alloc_fh(&mut self) -> u64 {
        let fh = self.next_fh;
        self.next_fh += 1;
        fh
    }

    /// Convert InodeMeta to fuser::FileAttr.
    fn meta_to_attr(meta: &InodeMeta) -> FileAttr {
        FileAttr {
            ino: meta.ino,
            size: meta.size,
            blocks: (meta.size + 511) / 512,
            atime: pair_to_system_time(meta.atime),
            mtime: pair_to_system_time(meta.mtime),
            ctime: pair_to_system_time(meta.ctime),
            crtime: pair_to_system_time(meta.crtime),
            kind: match meta.kind {
                InodeKind::File => FileType::RegularFile,
                InodeKind::Directory => FileType::Directory,
                InodeKind::Symlink => FileType::Symlink,
            },
            perm: meta.mode as u16,
            nlink: meta.nlink,
            uid: meta.uid,
            gid: meta.gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }
}

impl Filesystem for MdbFilesystem {
    fn init(
        &mut self,
        _req: &Request<'_>,
        _config: &mut fuser::KernelConfig,
    ) -> Result<(), libc::c_int> {
        debug!("MDBFS: init");
        Ok(())
    }

    fn destroy(&mut self) {
        debug!("MDBFS: destroy — flushing all data");
        if let Err(e) = self.store.flush() {
            error!("MDBFS: error flushing on destroy: {}", e);
        }
    }

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        debug!("MDBFS: lookup parent={} name={}", parent, name_str);

        match self.store.lookup_child(parent, name_str) {
            Some(ino) => {
                if let Some(meta) = self.store.get_inode(ino) {
                    reply.entry(&TTL, &Self::meta_to_attr(meta), 0);
                } else {
                    reply.error(ENOENT);
                }
            }
            None => reply.error(ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        debug!("MDBFS: getattr ino={}", ino);
        match self.store.get_inode(ino) {
            Some(meta) => reply.attr(&TTL, &Self::meta_to_attr(meta)),
            None => reply.error(ENOENT),
        }
    }

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        debug!("MDBFS: setattr ino={}", ino);

        // Handle truncation first (may need store operations)
        if let Some(new_size) = size {
            if let Some(meta) = self.store.get_inode(ino) {
                if meta.kind == InodeKind::File {
                    if let Err(e) = self.store.truncate_content(ino, new_size) {
                        error!("MDBFS: truncate error: {}", e);
                        reply.error(EIO);
                        return;
                    }
                }
            }
        }

        if let Some(meta) = self.store.get_inode_mut(ino) {
            if let Some(m) = mode {
                meta.mode = m;
            }
            if let Some(u) = uid {
                meta.uid = u;
            }
            if let Some(g) = gid {
                meta.gid = g;
            }
            if let Some(s) = size {
                meta.size = s;
            }

            let now_pair = system_time_to_pair(SystemTime::now());
            if let Some(a) = atime {
                meta.atime = match a {
                    TimeOrNow::SpecificTime(t) => system_time_to_pair(t),
                    TimeOrNow::Now => now_pair,
                };
            }
            if let Some(m) = mtime {
                meta.mtime = match m {
                    TimeOrNow::SpecificTime(t) => system_time_to_pair(t),
                    TimeOrNow::Now => now_pair,
                };
            }
            meta.ctime = now_pair;

            let attr = Self::meta_to_attr(meta);
            reply.attr(&TTL, &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn mknod(
        &mut self,
        req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!("MDBFS: mknod parent={} name={} mode={:o}", parent, name_str, mode);

        // Check parent is a directory
        if let Some(pmeta) = self.store.get_inode(parent) {
            if pmeta.kind != InodeKind::Directory {
                reply.error(ENOTDIR);
                return;
            }
            if pmeta.children.contains_key(name_str) {
                reply.error(EEXIST);
                return;
            }
        } else {
            reply.error(ENOENT);
            return;
        }

        let ino = self.store.alloc_ino();
        let now = system_time_to_pair(SystemTime::now());

        let meta = InodeMeta {
            ino,
            kind: InodeKind::File,
            size: 0,
            mode: mode & 0o7777,
            uid: req.uid(),
            gid: req.gid(),
            nlink: 1,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            children: std::collections::HashMap::new(),
            symlink_target: None,
            fold_depth: DEFAULT_FOLD_DEPTH,
            xattrs: std::collections::HashMap::new(),
        };

        let attr = Self::meta_to_attr(&meta);
        self.store.insert_inode(meta);

        // Add to parent directory
        if let Some(pmeta) = self.store.get_inode_mut(parent) {
            pmeta.children.insert(name_str.to_string(), ino);
            pmeta.mtime = now;
            pmeta.ctime = now;
        }

        reply.entry(&TTL, &attr, 0);
    }

    fn mkdir(
        &mut self,
        req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!("MDBFS: mkdir parent={} name={}", parent, name_str);

        if let Some(pmeta) = self.store.get_inode(parent) {
            if pmeta.kind != InodeKind::Directory {
                reply.error(ENOTDIR);
                return;
            }
            if pmeta.children.contains_key(name_str) {
                reply.error(EEXIST);
                return;
            }
        } else {
            reply.error(ENOENT);
            return;
        }

        let ino = self.store.alloc_ino();
        let now = system_time_to_pair(SystemTime::now());

        let meta = InodeMeta {
            ino,
            kind: InodeKind::Directory,
            size: 0,
            mode: mode & 0o7777,
            uid: req.uid(),
            gid: req.gid(),
            nlink: 2,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            children: std::collections::HashMap::new(),
            symlink_target: None,
            fold_depth: DEFAULT_FOLD_DEPTH,
            xattrs: std::collections::HashMap::new(),
        };

        let attr = Self::meta_to_attr(&meta);
        self.store.insert_inode(meta);

        // Add to parent and bump parent nlink
        if let Some(pmeta) = self.store.get_inode_mut(parent) {
            pmeta.children.insert(name_str.to_string(), ino);
            pmeta.nlink += 1;
            pmeta.mtime = now;
            pmeta.ctime = now;
        }

        reply.entry(&TTL, &attr, 0);
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!("MDBFS: unlink parent={} name={}", parent, name_str);

        let child_ino = match self.store.lookup_child(parent, name_str) {
            Some(ino) => ino,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        // Don't unlink directories
        if let Some(meta) = self.store.get_inode(child_ino) {
            if meta.kind == InodeKind::Directory {
                reply.error(EISDIR);
                return;
            }
        }

        // Remove from parent
        if let Some(pmeta) = self.store.get_inode_mut(parent) {
            pmeta.children.remove(name_str);
            let now = system_time_to_pair(SystemTime::now());
            pmeta.mtime = now;
            pmeta.ctime = now;
        }

        // Decrement nlink and remove if zero
        let should_remove = if let Some(meta) = self.store.get_inode_mut(child_ino) {
            meta.nlink = meta.nlink.saturating_sub(1);
            meta.nlink == 0
        } else {
            false
        };

        if should_remove {
            self.store.remove_inode(child_ino);
        }

        reply.ok();
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!("MDBFS: rmdir parent={} name={}", parent, name_str);

        let child_ino = match self.store.lookup_child(parent, name_str) {
            Some(ino) => ino,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        // Must be a directory and must be empty
        if let Some(meta) = self.store.get_inode(child_ino) {
            if meta.kind != InodeKind::Directory {
                reply.error(ENOTDIR);
                return;
            }
            if !meta.children.is_empty() {
                reply.error(ENOTEMPTY);
                return;
            }
        } else {
            reply.error(ENOENT);
            return;
        }

        // Remove from parent
        if let Some(pmeta) = self.store.get_inode_mut(parent) {
            pmeta.children.remove(name_str);
            pmeta.nlink = pmeta.nlink.saturating_sub(1);
            let now = system_time_to_pair(SystemTime::now());
            pmeta.mtime = now;
            pmeta.ctime = now;
        }

        self.store.remove_inode(child_ino);
        reply.ok();
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };
        let newname_str = match newname.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!(
            "MDBFS: rename parent={} name={} -> newparent={} newname={}",
            parent, name_str, newparent, newname_str
        );

        // Look up the source inode
        let src_ino = match self.store.lookup_child(parent, name_str) {
            Some(ino) => ino,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        // If destination exists, remove it first
        if let Some(dst_ino) = self.store.lookup_child(newparent, newname_str) {
            // Check if it's a non-empty directory
            if let Some(dst_meta) = self.store.get_inode(dst_ino) {
                if dst_meta.kind == InodeKind::Directory && !dst_meta.children.is_empty() {
                    reply.error(ENOTEMPTY);
                    return;
                }
            }
            self.store.remove_inode(dst_ino);
        }

        let now = system_time_to_pair(SystemTime::now());

        // Remove from old parent
        if let Some(pmeta) = self.store.get_inode_mut(parent) {
            pmeta.children.remove(name_str);
            pmeta.mtime = now;
            pmeta.ctime = now;
        }

        // Add to new parent
        if let Some(pmeta) = self.store.get_inode_mut(newparent) {
            pmeta.children.insert(newname_str.to_string(), src_ino);
            pmeta.mtime = now;
            pmeta.ctime = now;
        }

        // Update ctime on renamed inode
        if let Some(meta) = self.store.get_inode_mut(src_ino) {
            meta.ctime = now;
        }

        reply.ok();
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        debug!("MDBFS: open ino={} flags={}", ino, flags);

        match self.store.get_inode(ino) {
            Some(meta) if meta.kind == InodeKind::File => {
                let fh = self.alloc_fh();
                self.open_files.insert(fh, (ino, flags));
                reply.opened(fh, 0);
            }
            Some(_) => reply.error(EISDIR),
            None => reply.error(ENOENT),
        }
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        debug!("MDBFS: release ino={} fh={}", ino, fh);
        self.open_files.remove(&fh);

        // Flush on close
        if let Err(e) = self.store.flush() {
            error!("MDBFS: flush error on release: {}", e);
        }

        reply.ok();
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        debug!("MDBFS: read ino={} offset={} size={}", ino, offset, size);

        match self.store.read_content(ino) {
            Ok(data) => {
                let start = offset as usize;
                if start >= data.len() {
                    reply.data(&[]);
                } else {
                    let end = std::cmp::min(start + size as usize, data.len());
                    reply.data(&data[start..end]);
                }
            }
            Err(e) => {
                error!("MDBFS: read error for ino {}: {}", ino, e);
                reply.error(EIO);
            }
        }
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        debug!(
            "MDBFS: write ino={} offset={} len={}",
            ino,
            offset,
            data.len()
        );

        match self.store.write_content_range(ino, offset as u64, data) {
            Ok(()) => reply.written(data.len() as u32),
            Err(e) => {
                error!("MDBFS: write error for ino {}: {}", ino, e);
                reply.error(EIO);
            }
        }
    }

    fn flush(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        debug!("MDBFS: flush ino={}", ino);

        match self.store.flush() {
            Ok(()) => reply.ok(),
            Err(e) => {
                error!("MDBFS: flush error: {}", e);
                reply.error(EIO);
            }
        }
    }

    fn fsync(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        debug!("MDBFS: fsync ino={}", ino);
        match self.store.flush() {
            Ok(()) => reply.ok(),
            Err(e) => {
                error!("MDBFS: fsync error: {}", e);
                reply.error(EIO);
            }
        }
    }

    fn create(
        &mut self,
        req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!("MDBFS: create parent={} name={}", parent, name_str);

        // Check parent is directory
        if let Some(pmeta) = self.store.get_inode(parent) {
            if pmeta.kind != InodeKind::Directory {
                reply.error(ENOTDIR);
                return;
            }
            if pmeta.children.contains_key(name_str) {
                reply.error(EEXIST);
                return;
            }
        } else {
            reply.error(ENOENT);
            return;
        }

        let ino = self.store.alloc_ino();
        let now = system_time_to_pair(SystemTime::now());

        let meta = InodeMeta {
            ino,
            kind: InodeKind::File,
            size: 0,
            mode: mode & 0o7777,
            uid: req.uid(),
            gid: req.gid(),
            nlink: 1,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            children: std::collections::HashMap::new(),
            symlink_target: None,
            fold_depth: DEFAULT_FOLD_DEPTH,
            xattrs: std::collections::HashMap::new(),
        };

        let attr = Self::meta_to_attr(&meta);
        self.store.insert_inode(meta);

        // Add to parent directory
        if let Some(pmeta) = self.store.get_inode_mut(parent) {
            pmeta.children.insert(name_str.to_string(), ino);
            pmeta.mtime = now;
        }

        // Open the file
        let fh = self.alloc_fh();
        self.open_files.insert(fh, (ino, flags));

        reply.created(&TTL, &attr, 0, fh, 0);
    }

    fn opendir(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        debug!("MDBFS: opendir ino={}", ino);

        match self.store.get_inode(ino) {
            Some(meta) if meta.kind == InodeKind::Directory => {
                let fh = self.alloc_fh();
                self.open_dirs.insert(fh, ino);
                reply.opened(fh, 0);
            }
            Some(_) => reply.error(ENOTDIR),
            None => reply.error(ENOENT),
        }
    }

    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        _flags: i32,
        reply: ReplyEmpty,
    ) {
        debug!("MDBFS: releasedir ino={} fh={}", ino, fh);
        self.open_dirs.remove(&fh);
        reply.ok();
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        debug!("MDBFS: readdir ino={} offset={}", ino, offset);

        let meta = match self.store.get_inode(ino) {
            Some(m) => m.clone(),
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if meta.kind != InodeKind::Directory {
            reply.error(ENOTDIR);
            return;
        }

        // Build entries: ".", "..", then children
        let mut entries: Vec<(u64, FileType, String)> = Vec::new();
        entries.push((ino, FileType::Directory, ".".to_string()));
        entries.push((ino, FileType::Directory, "..".to_string()));

        // Sort children for deterministic ordering
        let mut children: Vec<_> = meta.children.iter().collect();
        children.sort_by_key(|(name, _)| name.clone());

        for (name, &child_ino) in &children {
            if let Some(child_meta) = self.store.get_inode(child_ino) {
                let ft = match child_meta.kind {
                    InodeKind::File => FileType::RegularFile,
                    InodeKind::Directory => FileType::Directory,
                    InodeKind::Symlink => FileType::Symlink,
                };
                entries.push((child_ino, ft, name.to_string()));
            }
        }

        for (i, (child_ino, ft, name)) in entries.iter().enumerate().skip(offset as usize) {
            // reply.add returns true if the buffer is full
            if reply.add(*child_ino, (i + 1) as i64, *ft, name) {
                break;
            }
        }

        reply.ok();
    }

    fn symlink(
        &mut self,
        req: &Request<'_>,
        parent: u64,
        link_name: &OsStr,
        target: &std::path::Path,
        reply: ReplyEntry,
    ) {
        let name_str = match link_name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };
        let target_str = match target.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!("MDBFS: symlink parent={} name={} -> {}", parent, name_str, target_str);

        if let Some(pmeta) = self.store.get_inode(parent) {
            if pmeta.kind != InodeKind::Directory {
                reply.error(ENOTDIR);
                return;
            }
            if pmeta.children.contains_key(name_str) {
                reply.error(EEXIST);
                return;
            }
        } else {
            reply.error(ENOENT);
            return;
        }

        let ino = self.store.alloc_ino();
        let now = system_time_to_pair(SystemTime::now());

        let meta = InodeMeta {
            ino,
            kind: InodeKind::Symlink,
            size: target_str.len() as u64,
            mode: 0o777,
            uid: req.uid(),
            gid: req.gid(),
            nlink: 1,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            children: std::collections::HashMap::new(),
            symlink_target: Some(target_str.to_string()),
            fold_depth: DEFAULT_FOLD_DEPTH,
            xattrs: std::collections::HashMap::new(),
        };

        let attr = Self::meta_to_attr(&meta);
        self.store.insert_inode(meta);

        if let Some(pmeta) = self.store.get_inode_mut(parent) {
            pmeta.children.insert(name_str.to_string(), ino);
            pmeta.mtime = now;
        }

        reply.entry(&TTL, &attr, 0);
    }

    fn readlink(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyData) {
        debug!("MDBFS: readlink ino={}", ino);

        match self.store.get_inode(ino) {
            Some(meta) => {
                if let Some(target) = &meta.symlink_target {
                    reply.data(target.as_bytes());
                } else {
                    reply.error(EINVAL);
                }
            }
            None => reply.error(ENOENT),
        }
    }

    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyStatfs) {
        let stats = self.store.statfs();
        reply.statfs(
            stats.blocks,
            stats.bfree,
            stats.bavail,
            stats.files,
            stats.ffree,
            stats.bsize,
            stats.namelen,
            stats.bsize,
        );
    }

    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!("MDBFS: setxattr ino={} name={}", ino, name_str);

        if let Some(meta) = self.store.get_inode_mut(ino) {
            meta.xattrs.insert(name_str.to_string(), value.to_vec());
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        size: u32,
        reply: ReplyXattr,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        debug!("MDBFS: getxattr ino={} name={} size={}", ino, name_str, size);

        // Handle special xattrs for MDB metadata
        if name_str == "mdb.fold_depth" {
            if let Some(meta) = self.store.get_inode(ino) {
                let val = meta.fold_depth.to_string();
                if size == 0 {
                    reply.size(val.len() as u32);
                } else if size < val.len() as u32 {
                    reply.error(ERANGE);
                } else {
                    reply.data(val.as_bytes());
                }
                return;
            }
        }

        if name_str == "mdb.address" {
            if let Some(meta) = self.store.get_inode(ino) {
                if meta.kind == InodeKind::File {
                    match self.store.read_content(ino) {
                        Ok(data) => {
                            let addr = mdb_core::coordinates::DimensionalAddress::from_bytes(&data);
                            let val = format!("n={} D4_spacetime={:.6} D5_momentum={}", addr.n, addr.d4_spacetime, addr.d5_momentum);
                            if size == 0 {
                                reply.size(val.len() as u32);
                            } else if size < val.len() as u32 {
                                reply.error(ERANGE);
                            } else {
                                reply.data(val.as_bytes());
                            }
                            return;
                        }
                        Err(_) => {
                            reply.error(EIO);
                            return;
                        }
                    }
                }
            }
        }

        if let Some(meta) = self.store.get_inode(ino) {
            if let Some(val) = meta.xattrs.get(name_str) {
                if size == 0 {
                    reply.size(val.len() as u32);
                } else if size < val.len() as u32 {
                    reply.error(ERANGE);
                } else {
                    reply.data(val);
                }
            } else {
                reply.error(ENODATA);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn removexattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(EINVAL);
                return;
            }
        };

        if let Some(meta) = self.store.get_inode_mut(ino) {
            if meta.xattrs.remove(name_str).is_some() {
                reply.ok();
            } else {
                reply.error(ENODATA);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn access(&mut self, _req: &Request<'_>, ino: u64, _mask: i32, reply: ReplyEmpty) {
        if self.store.get_inode(ino).is_some() {
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
}
