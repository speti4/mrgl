//! Handles a test holds open so that the code under test fails on the path: a read, a move or a removal of
//! a file or folder that another process has open without sharing. The lock lasts as long as the returned
//! `File`; a test drops it to let the path go.

use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

const FILE_SHARE_READ: u32 = 0x1;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

/// A file held open with no sharing: no other handle reads, writes, renames or removes it.
pub fn file(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .expect("a handle with no sharing")
}

/// A file held open that shares reads only: it can be read, not written over, renamed or removed.
pub fn file_sharing_reads(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
        .expect("a handle that shares reads")
}

/// A folder held open with no sharing (a folder handle needs the backup semantics flag): it can be neither
/// listed nor moved.
pub fn folder(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .expect("a folder handle with no sharing")
}
