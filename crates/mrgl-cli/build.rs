//! Gives `mrgl.exe` the window's icon (`crates/mrgl-app/icons/icon.ico`, drawn by `scripts/make-icon.py`)
//! and the workspace version in its file properties as Windows resources, with the standard library alone:
//! one compiled resource file (`.res`, the layout `rc.exe` writes) holds the `.ico` rewritten as one
//! `RT_ICON` per image and the `RT_GROUP_ICON` that lists them (task 012), and the `VS_VERSION_INFO` block of
//! an `RT_VERSION` resource with the package's version (task 079), and it is handed to the MSVC linker,
//! which takes a `.res` as an input file. No resource compiler and no build crate is needed; other targets
//! skip it.

use std::env;
use std::fs;
use std::path::PathBuf;

const ICON: &str = "../mrgl-app/icons/icon.ico";
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
/// MOVEABLE | DISCARDABLE, and PURE for the group, as `rc.exe` marks icon resources.
const FLAGS_ICON: u16 = 0x1010;
const FLAGS_GROUP: u16 = 0x1030;
/// MOVEABLE | PURE, as `rc.exe` marks a version resource.
const FLAGS_VERSION: u16 = 0x0030;
/// LANG_NEUTRAL, SUBLANG_NEUTRAL: the icon is the same in every language.
const LANGUAGE_NEUTRAL: u16 = 0;
/// English (United States), the language of the version strings, and Unicode (code page 1200).
const LANGUAGE_EN_US: u16 = 0x0409;
const CODE_PAGE_UNICODE: u16 = 0x04B0;
/// The signature and the structure version that start `VS_FIXEDFILEINFO`.
const FIXED_FILE_INFO_SIGNATURE: u32 = 0xFEEF_04BD;
const FIXED_FILE_INFO_VERSION: u32 = 0x0001_0000;
/// The flags `VS_FIXEDFILEINFO` defines (`VS_FFI_FILEFLAGSMASK`) and the prerelease one among them.
const FILE_FLAGS_MASK: u32 = 0x3F;
const FILE_FLAG_PRERELEASE: u32 = 0x2;
/// `VOS_NT_WINDOWS32` and `VFT_APP`: a Win32 application.
const FILE_OS: u32 = 0x0004_0004;
const FILE_TYPE_APP: u32 = 0x1;

fn main() {
    println!("cargo::rerun-if-changed={ICON}");
    println!("cargo::rerun-if-changed=build.rs");
    let windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if !(windows && msvc) {
        return;
    }
    let ico = fs::read(ICON).expect("read crates/mrgl-app/icons/icon.ico");
    let mut res = Vec::new();
    push_entry(&mut res, 0, 0, 0, LANGUAGE_NEUTRAL, &[]);
    icon_resource(&mut res, &ico);
    let version = Version::of_package();
    push_entry(
        &mut res,
        RT_VERSION,
        1,
        FLAGS_VERSION,
        LANGUAGE_EN_US,
        &version_info(&version),
    );
    let out =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo")).join("mrgl.res");
    fs::write(&out, res).expect("write mrgl.res");
    println!("cargo::rustc-link-arg-bins={}", out.display());
}

/// The resources of an `.ico` file, after the empty entry every `.res` starts with: each image as an
/// `RT_ICON` numbered from 1, and group icon 1 that lists them (the first group icon is the one Explorer
/// shows).
fn icon_resource(res: &mut Vec<u8>, ico: &[u8]) {
    let u16_at = |at: usize| u16::from_le_bytes([ico[at], ico[at + 1]]);
    let u32_at = |at: usize| u32::from_le_bytes([ico[at], ico[at + 1], ico[at + 2], ico[at + 3]]);
    assert!(
        ico.len() >= 6 && u16_at(0) == 0 && u16_at(2) == 1,
        "icon.ico is not an icon file"
    );
    let count = u16_at(4);

    let mut group = Vec::new();
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&count.to_le_bytes());
    for index in 0..count {
        // ICONDIRENTRY: width, height, colors, reserved, planes, bits per pixel, size, offset.
        let entry = 6 + 16 * usize::from(index);
        let size = u32_at(entry + 8) as usize;
        let offset = u32_at(entry + 12) as usize;
        let image = &ico[offset..offset + size];
        let id = index + 1;
        push_entry(res, RT_ICON, id, FLAGS_ICON, LANGUAGE_NEUTRAL, image);
        // GRPICONDIRENTRY: the same first 12 bytes, then the RT_ICON id in place of the offset.
        group.extend_from_slice(&ico[entry..entry + 12]);
        group.extend_from_slice(&id.to_le_bytes());
    }
    push_entry(res, RT_GROUP_ICON, 1, FLAGS_GROUP, LANGUAGE_NEUTRAL, &group);
}

/// The package's version as cargo hands it to the build script: the numbers and the prerelease suffix.
struct Version {
    text: String,
    major: u16,
    minor: u16,
    patch: u16,
    prerelease: bool,
}

impl Version {
    fn of_package() -> Version {
        let number = |name: &str| {
            env::var(name)
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or_else(|| panic!("{name} is a version number below 65536"))
        };
        Version {
            text: env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is set by cargo"),
            major: number("CARGO_PKG_VERSION_MAJOR"),
            minor: number("CARGO_PKG_VERSION_MINOR"),
            patch: number("CARGO_PKG_VERSION_PATCH"),
            prerelease: env::var("CARGO_PKG_VERSION_PRE").is_ok_and(|pre| !pre.is_empty()),
        }
    }
}

/// The `VS_VERSION_INFO` block: `VS_FIXEDFILEINFO` with the version as `major.minor.patch.0` for the file
/// and the product, then the strings the file's properties show (the version as written in `Cargo.toml`,
/// a prerelease suffix included) in one US English Unicode table, and the translation that names it.
fn version_info(version: &Version) -> Vec<u8> {
    let most = (u32::from(version.major) << 16) | u32::from(version.minor);
    let least = u32::from(version.patch) << 16;
    let flags = if version.prerelease {
        FILE_FLAG_PRERELEASE
    } else {
        0
    };
    let mut fixed = Vec::new();
    for value in [
        FIXED_FILE_INFO_SIGNATURE,
        FIXED_FILE_INFO_VERSION,
        most,
        least,
        most,
        least,
        FILE_FLAGS_MASK,
        flags,
        FILE_OS,
        FILE_TYPE_APP,
        0, // the subtype
        0, // the file date, most significant
        0, // and least significant
    ] {
        fixed.extend_from_slice(&value.to_le_bytes());
    }
    let strings: Vec<Vec<u8>> = [
        ("FileDescription", "mrgl command line"),
        ("FileVersion", version.text.as_str()),
        ("InternalName", "mrgl"),
        ("OriginalFilename", "mrgl.exe"),
        ("ProductName", "mrgl"),
        ("ProductVersion", version.text.as_str()),
    ]
    .into_iter()
    .map(|(key, value)| {
        let text = utf16(value);
        block(key, &text, text.len() / 2, true, &[])
    })
    .collect();
    let table_key = format!("{LANGUAGE_EN_US:04X}{CODE_PAGE_UNICODE:04X}");
    let table = block(&table_key, &[], 0, true, &strings);
    let string_file_info = block("StringFileInfo", &[], 0, true, &[table]);
    let mut translation = Vec::new();
    translation.extend_from_slice(&LANGUAGE_EN_US.to_le_bytes());
    translation.extend_from_slice(&CODE_PAGE_UNICODE.to_le_bytes());
    let var = block("Translation", &translation, translation.len(), false, &[]);
    let var_file_info = block("VarFileInfo", &[], 0, true, &[var]);
    block(
        "VS_VERSION_INFO",
        &fixed,
        fixed.len(),
        false,
        &[string_file_info, var_file_info],
    )
}

/// One block of a version resource: `wLength`, `wValueLength` (in characters for text, in bytes for
/// binary), `wType` (1 text, 0 binary), the UTF-16 key with its terminator, the value and the children,
/// each started on a four-byte boundary; `wLength` counts all of it but the padding after the last child.
fn block(
    key: &str,
    value: &[u8],
    value_length: usize,
    text: bool,
    children: &[Vec<u8>],
) -> Vec<u8> {
    let mut bytes = vec![0; 6];
    bytes.extend_from_slice(&utf16(key));
    pad4(&mut bytes);
    bytes.extend_from_slice(value);
    for child in children {
        pad4(&mut bytes);
        bytes.extend_from_slice(child);
    }
    let length = u16::try_from(bytes.len()).expect("a version block fits 64 KiB");
    let value_length = u16::try_from(value_length).expect("a version value fits 64 KiB");
    bytes[0..2].copy_from_slice(&length.to_le_bytes());
    bytes[2..4].copy_from_slice(&value_length.to_le_bytes());
    bytes[4..6].copy_from_slice(&u16::from(text).to_le_bytes());
    bytes
}

/// UTF-16 little-endian with the terminator.
fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn pad4(bytes: &mut Vec<u8>) {
    bytes.resize(bytes.len().next_multiple_of(4), 0);
}

/// One resource with a numbered type and name: the 32-byte header, then the data padded to four bytes.
fn push_entry(res: &mut Vec<u8>, kind: u16, name: u16, flags: u16, language: u16, data: &[u8]) {
    let size = u32::try_from(data.len()).expect("a resource fits 4 GiB");
    res.extend_from_slice(&size.to_le_bytes());
    res.extend_from_slice(&32u32.to_le_bytes());
    for ordinal in [kind, name] {
        res.extend_from_slice(&0xFFFFu16.to_le_bytes());
        res.extend_from_slice(&ordinal.to_le_bytes());
    }
    res.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    res.extend_from_slice(&flags.to_le_bytes());
    res.extend_from_slice(&language.to_le_bytes());
    res.extend_from_slice(&0u32.to_le_bytes()); // Version
    res.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    res.extend_from_slice(data);
    res.resize(res.len().next_multiple_of(4), 0);
}
