//! Gives `mrgl.exe` the window's icon (`crates/mrgl-app/icons/icon.ico`, drawn by `scripts/make-icon.py`) as
//! a Windows resource, with the standard library alone: the `.ico` is rewritten as a compiled resource file
//! (`.res`: one `RT_ICON` per image and the `RT_GROUP_ICON` that lists them, the layout `rc.exe` writes) and
//! handed to the MSVC linker, which takes a `.res` as an input file. No resource compiler and no build crate
//! is needed; other targets skip it.

use std::env;
use std::fs;
use std::path::PathBuf;

const ICON: &str = "../mrgl-app/icons/icon.ico";
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
/// MOVEABLE | DISCARDABLE, and PURE for the group, as `rc.exe` marks icon resources.
const FLAGS_ICON: u16 = 0x1010;
const FLAGS_GROUP: u16 = 0x1030;
/// LANG_NEUTRAL, SUBLANG_NEUTRAL: the icon is the same in every language.
const LANGUAGE: u16 = 0;

fn main() {
    println!("cargo::rerun-if-changed={ICON}");
    println!("cargo::rerun-if-changed=build.rs");
    let windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if !(windows && msvc) {
        return;
    }
    let ico = fs::read(ICON).expect("read crates/mrgl-app/icons/icon.ico");
    let res = icon_resource(&ico);
    let out =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo")).join("icon.res");
    fs::write(&out, res).expect("write icon.res");
    println!("cargo::rustc-link-arg-bins={}", out.display());
}

/// The `.res` file for an `.ico` file: the empty entry every `.res` starts with, each image as an `RT_ICON`
/// numbered from 1, and group icon 1 that lists them (the first group icon is the one Explorer shows).
fn icon_resource(ico: &[u8]) -> Vec<u8> {
    let u16_at = |at: usize| u16::from_le_bytes([ico[at], ico[at + 1]]);
    let u32_at = |at: usize| u32::from_le_bytes([ico[at], ico[at + 1], ico[at + 2], ico[at + 3]]);
    assert!(
        ico.len() >= 6 && u16_at(0) == 0 && u16_at(2) == 1,
        "icon.ico is not an icon file"
    );
    let count = u16_at(4);

    let mut res = Vec::new();
    push_entry(&mut res, 0, 0, 0, &[]);
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
        push_entry(&mut res, RT_ICON, id, FLAGS_ICON, image);
        // GRPICONDIRENTRY: the same first 12 bytes, then the RT_ICON id in place of the offset.
        group.extend_from_slice(&ico[entry..entry + 12]);
        group.extend_from_slice(&id.to_le_bytes());
    }
    push_entry(&mut res, RT_GROUP_ICON, 1, FLAGS_GROUP, &group);
    res
}

/// One resource with a numbered type and name: the 32-byte header, then the data padded to four bytes.
fn push_entry(res: &mut Vec<u8>, kind: u16, name: u16, flags: u16, data: &[u8]) {
    let size = u32::try_from(data.len()).expect("an icon image fits a resource");
    res.extend_from_slice(&size.to_le_bytes());
    res.extend_from_slice(&32u32.to_le_bytes());
    for ordinal in [kind, name] {
        res.extend_from_slice(&0xFFFFu16.to_le_bytes());
        res.extend_from_slice(&ordinal.to_le_bytes());
    }
    res.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    res.extend_from_slice(&flags.to_le_bytes());
    res.extend_from_slice(&LANGUAGE.to_le_bytes());
    res.extend_from_slice(&0u32.to_le_bytes()); // Version
    res.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    res.extend_from_slice(data);
    res.resize(res.len().next_multiple_of(4), 0);
}
