//! The file version of a Windows executable (task 019), read from its version resource by a parser of
//! our own: the DOS header, the PE signature and COFF header, the optional header's data directories, the
//! section table, the resource directory tree down to `RT_VERSION`, and the `VS_VERSIONINFO` block in it.
//! No Windows API and no crate is involved, so the fixture test runs on every target. Only the bytes the
//! walk touches are read: a client exe is tens of megabytes and its version resource a few hundred bytes.
//!
//! The version is the `FileVersion` string of the resource's `StringFileInfo` (free text, so a build
//! number above 65535 survives), and the `dwFileVersion` of `VS_FIXEDFILEINFO` when the string is missing
//! or unreadable (its four parts are 16-bit each). [`crate::flavor`] reads it when `.build.info` has no
//! row for a flavor.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::flavor::BuildVersion;

/// The resource type of a version resource.
const RT_VERSION: u32 = 16;
/// The signature at the start of `VS_FIXEDFILEINFO`.
const FIXED_FILE_INFO_SIGNATURE: u32 = 0xFEEF_04BD;
/// The size of `VS_FIXEDFILEINFO`.
const FIXED_FILE_INFO_SIZE: usize = 52;
/// `VS_VERSION_INFO` as the block's key, UTF-16 with its terminator.
const VERSION_INFO_KEY: &[u8] = b"V\0S\0_\0V\0E\0R\0S\0I\0O\0N\0_\0I\0N\0F\0O\0\0\0";
/// The largest version resource read; a real one is under 64 KiB, its `wLength` being 16-bit.
const MAX_VERSION_RESOURCE: u32 = 1 << 20;
/// The most entries one resource directory is allowed; a real one has a handful.
const MAX_DIRECTORY_ENTRIES: usize = 4096;
/// The most sections a PE file may have, per the format.
const MAX_SECTIONS: usize = 96;

/// The file version of an executable on disk, or the plain reason it cannot be read.
pub fn read_file_version(path: &Path) -> Result<BuildVersion, String> {
    let mut file = std::fs::File::open(path).map_err(|source| format!("cannot open: {source}"))?;
    file_version(&mut file)
}

/// The file version of an executable read through `reader` (a file, or a cursor over bytes in tests).
pub fn file_version<R: Read + Seek>(reader: &mut R) -> Result<BuildVersion, String> {
    let mut pe = Pe { reader };
    // The magic first, on its own: a text file named like the client is "not an executable", not
    // "truncated".
    if pe.read(0, 2, "the DOS header").ok().as_deref() != Some(b"MZ") {
        return Err("not an executable (no MZ header)".to_string());
    }
    let dos = pe.read(0, 0x40, "the DOS header")?;
    let pe_at = u64::from(u32_at(&dos, 0x3c));
    let coff = pe.read(pe_at, 24, "the PE header")?;
    if &coff[..4] != b"PE\0\0" {
        return Err("not a PE executable (no PE signature)".to_string());
    }
    let section_count = usize::from(u16_at(&coff, 6));
    let optional_size = usize::from(u16_at(&coff, 20));
    let optional_at = pe_at + 24;
    let optional = pe.read(optional_at, optional_size, "the optional header")?;
    if optional.len() < 2 {
        return Err("no optional header".to_string());
    }
    let directories_at = match u16_at(&optional, 0) {
        0x10b => 96,
        0x20b => 112,
        other => return Err(format!("unknown optional header magic 0x{other:x}")),
    };
    let entry_at = directories_at + 2 * 8;
    if optional.len() < entry_at + 8 {
        return Err("no resource directory entry in the optional header".to_string());
    }
    let resources_rva = u32_at(&optional, entry_at);
    let resources_size = u32_at(&optional, entry_at + 4);
    if resources_rva == 0 || resources_size == 0 {
        return Err("no resource section".to_string());
    }
    if section_count == 0 || section_count > MAX_SECTIONS {
        return Err(format!("{section_count} sections"));
    }
    let table = pe.read(
        optional_at + optional_size as u64,
        section_count * 40,
        "the section table",
    )?;
    let sections: Vec<Section> = table
        .as_chunks::<40>()
        .0
        .iter()
        .map(|header| Section {
            virtual_size: u32_at(header, 8),
            virtual_address: u32_at(header, 12),
            raw_size: u32_at(header, 16),
            raw_pointer: u32_at(header, 20),
        })
        .collect();
    let resources_at = rva_to_offset(&sections, resources_rva)
        .ok_or_else(|| "the resource directory lies in no section".to_string())?;

    let version_type = pe
        .directory_entries(resources_at, 0)?
        .into_iter()
        .find(|entry| entry.id == Some(RT_VERSION) && entry.is_directory)
        .ok_or_else(|| "no version resource (RT_VERSION)".to_string())?;
    // Below the type: the resource's name, then its language; the first of each is the one. A directory
    // where a leaf is expected (or the reverse) is followed either way, a few levels at most.
    let mut entry = version_type;
    let mut depth = 0;
    while entry.is_directory {
        depth += 1;
        if depth > 4 {
            return Err("the version resource tree is deeper than the format allows".to_string());
        }
        entry = pe
            .directory_entries(resources_at, entry.offset)?
            .into_iter()
            .next()
            .ok_or_else(|| "an empty directory in the version resource tree".to_string())?;
    }
    let data_entry = pe.read(
        resources_at + u64::from(entry.offset),
        16,
        "the version resource's data entry",
    )?;
    let data_rva = u32_at(&data_entry, 0);
    let data_size = u32_at(&data_entry, 4);
    if data_size == 0 || data_size > MAX_VERSION_RESOURCE {
        return Err(format!("a version resource of {data_size} bytes"));
    }
    let data_at = rva_to_offset(&sections, data_rva)
        .ok_or_else(|| "the version resource lies in no section".to_string())?;
    let block = pe.read(data_at, data_size as usize, "the version resource")?;
    version_of_block(&block)
}

/// The file version in a `VS_VERSIONINFO` block: the `FileVersion` string, else `VS_FIXEDFILEINFO`.
pub fn version_of_block(bytes: &[u8]) -> Result<BuildVersion, String> {
    let (root, _) = Block::parse(bytes)
        .filter(|(root, _)| root.key == VERSION_INFO_KEY)
        .ok_or_else(|| "the version resource is not a VS_VERSIONINFO block".to_string())?;
    if let Some(text) = file_version_string(&root)
        && let Some(version) = BuildVersion::parse(&version_text(&text))
    {
        return Ok(version);
    }
    if root.value.len() < FIXED_FILE_INFO_SIZE {
        return Err(
            "no FileVersion string and no VS_FIXEDFILEINFO in the version resource".to_string(),
        );
    }
    if u32_at(root.value, 0) != FIXED_FILE_INFO_SIGNATURE {
        return Err(
            "no FileVersion string and a VS_FIXEDFILEINFO with a bad signature".to_string(),
        );
    }
    let most = u32_at(root.value, 8);
    let least = u32_at(root.value, 12);
    Ok(BuildVersion {
        major: most >> 16,
        minor: most & 0xffff,
        patch: least >> 16,
        build: least & 0xffff,
    })
}

/// The `FileVersion` string of the first string table under `StringFileInfo`, as text.
fn file_version_string(root: &Block<'_>) -> Option<String> {
    let string_file_info = root
        .children()
        .find(|block| block.key_is("StringFileInfo"))?;
    let table = string_file_info.children().next()?;
    let string = table.children().find(|block| block.key_is("FileVersion"))?;
    Some(utf16_text(string.value))
}

/// The leading version of a `FileVersion` string: `1, 15, 7, 61582`, `v1.15.7.61582` and
/// `1.15.7.61582 (build notes)` all read as `1.15.7.61582`.
fn version_text(text: &str) -> String {
    let text: String = text
        .chars()
        .map(|c| if c == ',' { '.' } else { c })
        .filter(|c| !c.is_whitespace())
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    text.trim_end_matches('.').to_string()
}

/// One block of a version resource: `wLength`, `wValueLength`, `wType`, a UTF-16 key, the value and the
/// children, each 4-byte aligned.
struct Block<'a> {
    /// The key, UTF-16 with its terminator.
    key: &'a [u8],
    value: &'a [u8],
    children: &'a [u8],
}

impl<'a> Block<'a> {
    /// The block at the start of `bytes` and what follows it (its siblings), or `None` when the bytes
    /// are not one.
    fn parse(bytes: &'a [u8]) -> Option<(Block<'a>, &'a [u8])> {
        let length = usize::from(u16_at_checked(bytes, 0)?);
        let value_length = usize::from(u16_at_checked(bytes, 2)?);
        let value_is_text = u16_at_checked(bytes, 4)? == 1;
        let block = bytes.get(..length.min(bytes.len()))?;
        if length < 6 {
            return None;
        }
        let key_end = (6..block.len())
            .step_by(2)
            .find(|&at| block.get(at..at + 2) == Some(&[0, 0]))?
            + 2;
        let key = &block[6..key_end];
        let value_at = align4(key_end);
        let value_bytes = if value_is_text {
            value_length * 2
        } else {
            value_length
        };
        let value = block.get(value_at..value_at + value_bytes)?;
        let children_at = align4(value_at + value_bytes).min(block.len());
        let children = &block[children_at..];
        let next = &bytes[align4(length).min(bytes.len())..];
        Some((
            Block {
                key,
                value,
                children,
            },
            next,
        ))
    }

    /// Whether the key is this ASCII name.
    fn key_is(&self, name: &str) -> bool {
        self.key.len() == (name.len() + 1) * 2
            && name
                .bytes()
                .chain([0])
                .zip(self.key.as_chunks::<2>().0)
                .all(|(byte, unit)| *unit == [byte, 0])
    }

    /// The child blocks, in order, stopping at the first thing that is not a block.
    fn children(&self) -> impl Iterator<Item = Block<'a>> {
        let mut rest = self.children;
        std::iter::from_fn(move || {
            let (child, next) = Block::parse(rest)?;
            rest = next;
            Some(child)
        })
    }
}

/// A UTF-16 value as text, without its terminator.
fn utf16_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|unit| u16::from_le_bytes(*unit))
        .take_while(|unit| *unit != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// A section's placement, for mapping an RVA to a file offset.
struct Section {
    virtual_size: u32,
    virtual_address: u32,
    raw_size: u32,
    raw_pointer: u32,
}

/// The file offset of an RVA, through the section that holds it.
fn rva_to_offset(sections: &[Section], rva: u32) -> Option<u64> {
    sections.iter().find_map(|section| {
        let size = section.virtual_size.max(section.raw_size);
        let within = rva.checked_sub(section.virtual_address)?;
        (within < size).then(|| u64::from(section.raw_pointer) + u64::from(within))
    })
}

/// One entry of a resource directory.
struct DirectoryEntry {
    /// The integer id; `None` for a named entry.
    id: Option<u32>,
    /// The offset from the start of the resource section, to a directory or to a data entry.
    offset: u32,
    is_directory: bool,
}

/// The reads of one walk, each a seek and an exact read.
struct Pe<'a, R> {
    reader: &'a mut R,
}

impl<R: Read + Seek> Pe<'_, R> {
    fn read(&mut self, offset: u64, length: usize, what: &str) -> Result<Vec<u8>, String> {
        self.reader
            .seek(SeekFrom::Start(offset))
            .map_err(|source| format!("cannot seek to {what} at 0x{offset:x}: {source}"))?;
        let mut buffer = vec![0u8; length];
        self.reader
            .read_exact(&mut buffer)
            .map_err(|source| format!("cannot read {what} at 0x{offset:x}: {source}"))?;
        Ok(buffer)
    }

    /// The entries of the resource directory at `offset` from the resource section's start.
    fn directory_entries(
        &mut self,
        resources_at: u64,
        offset: u32,
    ) -> Result<Vec<DirectoryEntry>, String> {
        let at = resources_at + u64::from(offset);
        let header = self.read(at, 16, "a resource directory")?;
        let count = usize::from(u16_at(&header, 12)) + usize::from(u16_at(&header, 14));
        if count > MAX_DIRECTORY_ENTRIES {
            return Err(format!("a resource directory with {count} entries"));
        }
        let entries = self.read(at + 16, count * 8, "the resource directory entries")?;
        Ok(entries
            .as_chunks::<8>()
            .0
            .iter()
            .map(|entry| {
                let name = u32_at(entry, 0);
                let data = u32_at(entry, 4);
                DirectoryEntry {
                    id: (name & 0x8000_0000 == 0).then_some(name & 0xffff),
                    offset: data & 0x7fff_ffff,
                    is_directory: data & 0x8000_0000 != 0,
                }
            })
            .collect())
    }
}

/// A little-endian u16 in a buffer long enough (the reads are exact).
fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// A little-endian u16, or `None` past the end.
fn u16_at_checked(bytes: &[u8], at: usize) -> Option<u16> {
    let unit = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([unit[0], unit[1]]))
}

/// A little-endian u32 in a buffer long enough (the reads are exact).
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// The next multiple of four.
const fn align4(at: usize) -> usize {
    at.div_ceil(4) * 4
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    //! Over the stubs of `tests/fixtures/client-exe/` (written by `make-stubs.py` next to them) and
    //! copies of them bent out of shape: every failure is a reason, never a panic.

    use std::io::Cursor;

    use super::*;

    /// The Classic Era stub: PE32+, a `FileVersion` string of `1.15.7.70100` and a fixed info whose build
    /// is that number's low 16 bits.
    const ERA: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/client-exe/_classic_era_/WowClassic.exe"
    ));
    /// The classic stub: a resource section with an `RT_RCDATA` entry and no version resource.
    const NO_VERSION: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/client-exe/_classic_/WowClassic.exe"
    ));
    /// A PE32 (32-bit) stub with a version resource.
    const PE32: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/client-exe/stubs/pe32.exe"
    ));
    /// A stub whose version resource has the fixed info only, no string table.
    const FIXED_ONLY: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/client-exe/stubs/fixed-only.exe"
    ));

    fn version(bytes: &[u8]) -> Result<String, String> {
        file_version(&mut Cursor::new(bytes)).map(|version| version.to_string())
    }

    /// The offset of a byte pattern in a stub.
    fn find(bytes: &[u8], pattern: &[u8]) -> usize {
        bytes
            .windows(pattern.len())
            .position(|window| window == pattern)
            .expect("the pattern is in the stub")
    }

    #[test]
    fn the_string_wins_over_the_fixed_info() {
        assert_eq!(version(ERA), Ok("1.15.7.70100".to_string()));
        assert_eq!(version(PE32), Ok("3.4.3.51666".to_string()));
    }

    #[test]
    fn the_fixed_info_serves_without_a_string_table() {
        assert_eq!(version(FIXED_ONLY), Ok("2.5.4.44833".to_string()));
        // A string table whose FileVersion is not a version falls back to the fixed info too.
        let mut bent = ERA.to_vec();
        let at = find(
            &bent,
            &"1.15.7.70100"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<u8>>(),
        );
        for (index, byte) in "latest"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .enumerate()
        {
            bent[at + index] = byte;
        }
        assert_eq!(version(&bent), Ok("1.15.7.4564".to_string()));
    }

    #[test]
    fn not_an_executable_is_a_reason() {
        assert_eq!(
            version(b"#!/bin/sh\necho mrgl\n"),
            Err("not an executable (no MZ header)".to_string())
        );
        assert!(version(b"MZ").is_err_and(|reason| reason.contains("cannot read the DOS header")));
        let mut bent = ERA.to_vec();
        bent[0x40] = b'N';
        assert_eq!(
            version(&bent),
            Err("not a PE executable (no PE signature)".to_string())
        );
    }

    #[test]
    fn a_truncated_file_is_a_reason() {
        let reason = version(&ERA[..0x200]).expect_err("truncated");
        assert!(reason.starts_with("cannot read "), "{reason}");
        let reason = version(&ERA[..0x60]).expect_err("truncated");
        assert!(reason.contains("the optional header"), "{reason}");
    }

    #[test]
    fn no_resource_section_and_no_version_resource_are_reasons() {
        // PE32+: the optional header starts at 0x58, its data directories at +112, the resource one third.
        let mut bent = ERA.to_vec();
        for byte in &mut bent[0xd8..0xe0] {
            *byte = 0;
        }
        assert_eq!(version(&bent), Err("no resource section".to_string()));
        assert_eq!(
            version(NO_VERSION),
            Err("no version resource (RT_VERSION)".to_string())
        );
    }

    #[test]
    fn a_bad_fixed_info_is_a_reason() {
        let mut bent = FIXED_ONLY.to_vec();
        let at = find(&bent, &FIXED_FILE_INFO_SIGNATURE.to_le_bytes());
        bent[at] = 0;
        assert_eq!(
            version(&bent),
            Err("no FileVersion string and a VS_FIXEDFILEINFO with a bad signature".to_string())
        );
        let mut bent = FIXED_ONLY.to_vec();
        let at = find(&bent, VERSION_INFO_KEY);
        bent[at] = b'X';
        assert_eq!(
            version(&bent),
            Err("the version resource is not a VS_VERSIONINFO block".to_string())
        );
    }

    /// A copy of the Classic Era stub with little-endian numbers written over it.
    fn bent(writes: &[(usize, &[u8])]) -> Vec<u8> {
        let mut bytes = ERA.to_vec();
        for (at, value) in writes {
            bytes[*at..*at + value.len()].copy_from_slice(value);
        }
        bytes
    }

    // The stub's layout (`make-stubs.py`): the COFF header at 0x44 (its section count at +2, the optional
    // header's size at +16), the PE32+ optional header at 0x58 (the resource directory entry at +128), the
    // `.rsrc` section at 0x200 with the root directory (its count at +14 and its one entry at +16), the
    // type directory at 0x218, the name directory at 0x230 (its entry at +16), the data entry at 0x248.
    const SECTION_COUNT: usize = 0x46;
    const OPTIONAL_SIZE: usize = 0x54;
    const OPTIONAL: usize = 0x58;
    const RESOURCE_ENTRY: usize = 0xd8;
    const ROOT_COUNT: usize = 0x20e;
    const TYPE_COUNT: usize = 0x226;
    const NAME_ENTRY: usize = 0x240;
    const DATA_ENTRY: usize = 0x248;

    #[test]
    fn a_header_bent_out_of_shape_is_a_reason() {
        for (writes, reason) in [
            (
                vec![(OPTIONAL_SIZE, &0u16.to_le_bytes()[..])],
                "no optional header".to_string(),
            ),
            (
                vec![(OPTIONAL, &0u16.to_le_bytes()[..])],
                "unknown optional header magic 0x0".to_string(),
            ),
            (
                vec![(OPTIONAL_SIZE, &100u16.to_le_bytes()[..])],
                "no resource directory entry in the optional header".to_string(),
            ),
            (
                vec![(SECTION_COUNT, &0u16.to_le_bytes()[..])],
                "0 sections".to_string(),
            ),
            (
                vec![(SECTION_COUNT, &200u16.to_le_bytes()[..])],
                "200 sections".to_string(),
            ),
            (
                vec![(RESOURCE_ENTRY, &0x7fff_0000u32.to_le_bytes()[..])],
                "the resource directory lies in no section".to_string(),
            ),
        ] {
            assert_eq!(version(&bent(&writes)), Err(reason));
        }
    }

    #[test]
    fn a_resource_tree_bent_out_of_shape_is_a_reason() {
        let subdirectory = 0x8000_0000u32;
        for (writes, reason) in [
            (
                vec![(ROOT_COUNT, &5000u16.to_le_bytes()[..])],
                "a resource directory with 5000 entries".to_string(),
            ),
            (
                vec![(TYPE_COUNT, &0u16.to_le_bytes()[..])],
                "an empty directory in the version resource tree".to_string(),
            ),
            // The name directory's entry points back at the type directory: a loop.
            (
                vec![(NAME_ENTRY + 4, &(0x18 | subdirectory).to_le_bytes()[..])],
                "the version resource tree is deeper than the format allows".to_string(),
            ),
            (
                vec![(DATA_ENTRY + 4, &0u32.to_le_bytes()[..])],
                "a version resource of 0 bytes".to_string(),
            ),
            (
                vec![(
                    DATA_ENTRY + 4,
                    &(MAX_VERSION_RESOURCE + 1).to_le_bytes()[..],
                )],
                format!("a version resource of {} bytes", MAX_VERSION_RESOURCE + 1),
            ),
            (
                vec![(DATA_ENTRY, &0x7fff_0000u32.to_le_bytes()[..])],
                "the version resource lies in no section".to_string(),
            ),
        ] {
            assert_eq!(version(&bent(&writes)), Err(reason));
        }
    }

    #[test]
    fn a_version_block_with_neither_string_nor_fixed_info_is_a_reason() {
        // A `VS_VERSION_INFO` block with an empty value and no children: the header, the key, padding.
        let mut block = vec![0u8; 6];
        block.extend_from_slice(VERSION_INFO_KEY);
        block.resize(align4(block.len()), 0);
        let length = block.len() as u16;
        block[..2].copy_from_slice(&length.to_le_bytes());
        assert_eq!(
            version_of_block(&block),
            Err(
                "no FileVersion string and no VS_FIXEDFILEINFO in the version resource".to_string()
            )
        );
        assert!(
            Block::parse(&[4, 0, 0, 0, 0, 0, 0, 0]).is_none(),
            "a block shorter than its header is none"
        );
    }

    /// A reader whose seeks fail from an offset on: the DOS header reads, the PE header does not.
    struct SeekFails {
        inner: Cursor<&'static [u8]>,
        from: u64,
    }

    impl Read for SeekFails {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.inner.read(buf)
        }
    }

    impl Seek for SeekFails {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            match to {
                SeekFrom::Start(offset) if offset >= self.from => {
                    Err(std::io::Error::other("the device is gone"))
                }
                to => self.inner.seek(to),
            }
        }
    }

    #[test]
    fn a_seek_that_fails_is_a_reason() {
        let mut reader = SeekFails {
            inner: Cursor::new(ERA),
            from: 0x40,
        };
        assert_eq!(
            file_version(&mut reader),
            Err("cannot seek to the PE header at 0x40: the device is gone".to_string())
        );
    }

    #[test]
    fn a_block_walk_never_panics_on_garbage() {
        for length in 0..64 {
            let garbage: Vec<u8> = (0..length).map(|i| (i * 37 % 251) as u8).collect();
            let _ = version_of_block(&garbage);
            let _ = Block::parse(&garbage);
        }
        assert!(version_of_block(&[0xff; 64]).is_err());
    }

    #[test]
    fn a_version_string_reads_with_commas_or_dots() {
        assert_eq!(version_text("1, 15, 7, 61582"), "1.15.7.61582");
        assert_eq!(version_text("12.1.5.70448"), "12.1.5.70448");
        assert_eq!(version_text("v1.15.7 beta"), "1.15.7");
        assert_eq!(
            version_text("10.0.26100.8875 (WinBuild.160101.0800)"),
            "10.0.26100.8875"
        );
        assert_eq!(version_text("1.15.7."), "1.15.7");
        assert_eq!(version_text("latest"), "");
    }
}
