"""Writes the synthetic client exe stubs of this fixture root (task 019).

Each stub is a minimal PE file: a DOS header, the PE signature and COFF header, an optional header
(PE32+ or PE32) and one `.rsrc` section with a resource directory tree. It has no code, no entry point
and no imports, so it runs nothing; it is never a game executable or a copy of one. The version
resource is a `VS_VERSIONINFO` block with a `VS_FIXEDFILEINFO` and, unless left out on purpose, a
`StringFileInfo` whose `FileVersion` carries the version as text (the fixed info keeps 16-bit parts, so
a build number above 65535 survives only in the string; the parser reads the string first).

Deterministic: every timestamp is zero and every byte follows from the arguments, so a run rewrites the
same files. Run it from the repo root, with the stubs below as its only output:

    python -I crates/mrgl-core/tests/fixtures/client-exe/make-stubs.py
"""

from __future__ import annotations

import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent

RT_RCDATA = 10
RT_VERSION = 16
LANGUAGE = 0x0409  # en-US
CODE_PAGE = 0x04B0  # Unicode
FILE_ALIGNMENT = 0x200
SECTION_ALIGNMENT = 0x1000
RSRC_RVA = 0x1000
RSRC_RAW = 0x200


def pad4(data: bytes) -> bytes:
    return data + b"\0" * (-len(data) % 4)


def utf16z(text: str) -> bytes:
    return (text + "\0").encode("utf-16-le")


def block(key: str, value: bytes, children: list[bytes], text: bool = False) -> bytes:
    """One block of a version resource: header, key, aligned value, aligned children; `wLength`
    counts everything up to the last child's end (the padding between siblings is not part of a
    block)."""
    value_length = len(value) // 2 if text else len(value)
    head = struct.pack("<HHH", 0, value_length, 1 if text else 0) + utf16z(key)
    body = pad4(head) + value
    for child in children:
        body = pad4(body) + child
    return struct.pack("<H", len(body)) + body[2:]


def string_block(key: str, text: str) -> bytes:
    return block(key, utf16z(text), [], text=True)


def version_info(version: tuple[int, int, int, int], with_strings: bool) -> bytes:
    major, minor, patch, build = version
    most = (major << 16) | minor
    least = (patch << 16) | (build & 0xFFFF)
    fixed = struct.pack(
        "<13I",
        0xFEEF04BD,  # dwSignature
        0x00010000,  # dwStrucVersion
        most,  # dwFileVersionMS
        least,  # dwFileVersionLS
        most,  # dwProductVersionMS
        least,  # dwProductVersionLS
        0x3F,  # dwFileFlagsMask
        0,  # dwFileFlags
        0x00040004,  # dwFileOS: VOS_NT_WINDOWS32
        1,  # dwFileType: VFT_APP
        0,  # dwFileSubtype
        0,  # dwFileDateMS
        0,  # dwFileDateLS
    )
    children = []
    if with_strings:
        table = block(
            f"{LANGUAGE:04X}{CODE_PAGE:04X}",
            b"",
            [
                string_block("FileDescription", "mrgl test stub, not a game client"),
                string_block("FileVersion", ".".join(str(part) for part in version)),
                string_block("ProductVersion", ".".join(str(part) for part in version)),
            ],
        )
        children.append(block("StringFileInfo", b"", [table]))
    children.append(
        block(
            "VarFileInfo",
            b"",
            [block("Translation", struct.pack("<HH", LANGUAGE, CODE_PAGE), [])],
        )
    )
    return block("VS_VERSION_INFO", fixed, children)


def directory(entries: list[tuple[int, int]]) -> bytes:
    """A resource directory with id entries only: `(id, offset)`, the high bit of the offset set for a
    subdirectory."""
    header = struct.pack("<IIHHHH", 0, 0, 0, 0, 0, len(entries))
    return header + b"".join(struct.pack("<II", name, offset) for name, offset in entries)


def resource_section(resource_type: int, data: bytes) -> bytes:
    """A tree with one resource: type → name 1 → language, then the data entry and the data."""
    subdirectory = 0x80000000
    root = directory([(resource_type, 0x18 | subdirectory)])
    by_type = directory([(1, 0x30 | subdirectory)])
    by_name = directory([(LANGUAGE, 0x48)])
    assert len(root) == 0x18 and len(by_type) == 0x18 and len(by_name) == 0x18
    data_entry = struct.pack("<IIII", RSRC_RVA + 0x58, len(data), 0, 0)
    return root + by_type + by_name + data_entry + data


def pe(rsrc: bytes, plus: bool) -> bytes:
    """A PE32+ (or PE32) image with one `.rsrc` section holding `rsrc`."""
    rsrc_raw_size = len(rsrc) + (-len(rsrc) % FILE_ALIGNMENT)
    directories = [(0, 0)] * 16
    directories[2] = (RSRC_RVA, len(rsrc))
    if plus:
        optional = struct.pack(
            "<HBBIIIII",
            0x20B, 0, 0, 0, rsrc_raw_size, 0, 0, 0,
        ) + struct.pack(
            "<QIIHHHHHHIIIIHHQQQQII",
            0x140000000, SECTION_ALIGNMENT, FILE_ALIGNMENT, 6, 0, 0, 0, 6, 0, 0,
            SECTION_ALIGNMENT * 2, FILE_ALIGNMENT, 0, 2, 0,
            0x100000, 0x1000, 0x100000, 0x1000, 0, 16,
        )
    else:
        optional = struct.pack(
            "<HBBIIIIII",
            0x10B, 0, 0, 0, rsrc_raw_size, 0, 0, 0, 0,
        ) + struct.pack(
            "<IIIHHHHHHIIIIHHIIIIII",
            0x400000, SECTION_ALIGNMENT, FILE_ALIGNMENT, 6, 0, 0, 0, 6, 0, 0,
            SECTION_ALIGNMENT * 2, FILE_ALIGNMENT, 0, 2, 0,
            0x100000, 0x1000, 0x100000, 0x1000, 0, 16,
        )
    optional += b"".join(struct.pack("<II", rva, size) for rva, size in directories)
    assert len(optional) == (240 if plus else 224)
    machine = 0x8664 if plus else 0x14C
    characteristics = 0x22 if plus else 0x102  # executable image; large-address-aware or 32-bit
    coff = struct.pack("<HHIIIHH", machine, 1, 0, 0, 0, len(optional), characteristics)
    section = (
        b".rsrc\0\0\0"
        + struct.pack("<IIII", len(rsrc), RSRC_RVA, rsrc_raw_size, RSRC_RAW)
        + struct.pack("<IIHHI", 0, 0, 0, 0, 0x40000040)
    )
    dos = b"MZ" + b"\0" * 58 + struct.pack("<I", 0x40)
    headers = dos + b"PE\0\0" + coff + optional + section
    assert len(headers) <= RSRC_RAW
    headers += b"\0" * (RSRC_RAW - len(headers))
    return headers + rsrc + b"\0" * (rsrc_raw_size - len(rsrc))


STUBS = {
    # The Classic Era folder: no row in .build.info, the exe carries the version; the build number is
    # above 65535 so only the string holds it whole.
    "_classic_era_/WowClassic.exe": pe(
        resource_section(RT_VERSION, version_info((1, 15, 7, 70100), with_strings=True)), plus=True
    ),
    # The retail folder: .build.info has the row (12.1.5.70448) and wins over the exe.
    "_retail_/Wow.exe": pe(
        resource_section(RT_VERSION, version_info((12, 1, 0, 70000), with_strings=True)), plus=True
    ),
    # The classic folder: no row and an exe without a version resource, so the build stays unknown.
    "_classic_/WowClassic.exe": pe(resource_section(RT_RCDATA, b"mrgl"), plus=True),
    # For the parser's unit tests: a 32-bit image, and a version resource without a string table.
    "stubs/pe32.exe": pe(
        resource_section(RT_VERSION, version_info((3, 4, 3, 51666), with_strings=True)), plus=False
    ),
    "stubs/fixed-only.exe": pe(
        resource_section(RT_VERSION, version_info((2, 5, 4, 44833), with_strings=False)), plus=True
    ),
}


def main() -> None:
    for name, data in STUBS.items():
        path = HERE / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        print(f"{path.relative_to(HERE)}: {len(data)} bytes")


if __name__ == "__main__":
    main()
