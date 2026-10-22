"""Writes the synthetic fixture zips of task 004 next to this script, and the release asset the GitHub
source's folder stub serves (task 040) under `../releases/`, deterministically (fixed timestamps, fixed entry
order, deflate), so a rerun changes no byte. Run it as `python -I <this file>` after editing a shape below;
the zips are committed. Nothing here comes from a real addon."""

from pathlib import Path
import zipfile

HERE = Path(__file__).resolve().parent
RELEASES = HERE.parent / "releases"
EPOCH = (1980, 1, 1, 0, 0, 0)


def toc(name, version, interface="16001", extra=""):
    return (
        f"## Interface: {interface}\r\n## Title: {name}\r\n## Version: {version}\r\n"
        f"## Author: mrgl fixtures\r\n{extra}\r\ncore.lua\r\n"
    )


def write(file_name, entries):
    path = HERE / file_name
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in entries:
            info = zipfile.ZipInfo(name, date_time=EPOCH)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o644 << 16) if not name.endswith("/") else (0o755 << 16) | 0x10
            archive.writestr(info, content)
    print(path.name, [name for name, _ in entries])


def declare_size(file_name, entry, declared):
    """Rewrites the uncompressed size one entry declares, in its local header and in the central directory,
    and leaves its data and its CRC as they are: a zip that lies about its size (task 039)."""
    path = HERE / file_name
    data = bytearray(path.read_bytes())
    with zipfile.ZipFile(path) as archive:
        local = archive.getinfo(entry).header_offset
    assert data[local:local + 4] == b"PK\x03\x04"
    data[local + 22:local + 26] = declared.to_bytes(4, "little")
    central = data.find(b"PK\x01\x02")
    while central != -1:
        name_length = int.from_bytes(data[central + 28:central + 30], "little")
        if data[central + 46:central + 46 + name_length] == entry.encode():
            data[central + 24:central + 28] = declared.to_bytes(4, "little")
        central = data.find(b"PK\x01\x02", central + 46)
    path.write_bytes(bytes(data))
    print(" ", entry, "declares", declared, "bytes")


write("one-folder-1.0.0.zip", [
    ("OneFolder/", ""),
    ("OneFolder/OneFolder.toc", toc("One Folder", "1.0.0", extra="## Notes: one folder, version 1.0.0\r\n")),
    ("OneFolder/core.lua", "-- OneFolder 1.0.0\n"),
    ("OneFolder/old.lua", "-- dropped in 1.1.0\n"),
    ("OneFolder/locale/enUS.lua", "-- enUS\n"),
])

write("one-folder-1.1.0.zip", [
    ("OneFolder/", ""),
    ("OneFolder/OneFolder.toc", toc("One Folder", "1.1.0", extra="## Notes: one folder, version 1.1.0\r\n")),
    ("OneFolder/core.lua", "-- OneFolder 1.1.0\n"),
    ("OneFolder/new.lua", "-- new in 1.1.0\n"),
    ("OneFolder/locale/enUS.lua", "-- enUS\n"),
])

write("one-folder-1.2.0.zip", [
    ("OneFolder/OneFolder.toc", toc("One Folder", "1.2.0")),
    ("OneFolder/core.lua", "-- OneFolder 1.2.0\n"),
])

write("several-folders-2.0.0.zip", [
    ("README.md", "# Several folders\n\nA root file the install ignores.\n"),
    ("Docs/notes.txt", "a folder without a TOC, ignored\n"),
    ("SeveralFolders/SeveralFolders.toc", toc("Several Folders", "2.0.0")),
    ("SeveralFolders/core.lua", "-- SeveralFolders 2.0.0\n"),
    ("SeveralFolders_Options/SeveralFolders_Options.toc",
     toc("Several Folders: Options", "2.0.0", extra="## LoadOnDemand: 1\r\n## Dependencies: SeveralFolders\r\n")),
    ("SeveralFolders_Options/core.lua", "-- options 2.0.0\n"),
    ("SeveralFolders_Data/SeveralFolders_Data.toc", toc("Several Folders: Data", "2.0.0")),
    ("SeveralFolders_Data/core.lua", "-- data 2.0.0\n"),
])

write("several-folders-2.1.0.zip", [
    ("SeveralFolders/SeveralFolders.toc", toc("Several Folders", "2.1.0")),
    ("SeveralFolders/core.lua", "-- SeveralFolders 2.1.0\n"),
    ("SeveralFolders_Options/SeveralFolders_Options.toc",
     toc("Several Folders: Options", "2.1.0", extra="## LoadOnDemand: 1\r\n## Dependencies: SeveralFolders\r\n")),
    ("SeveralFolders_Options/core.lua", "-- options 2.1.0\n"),
])

write("root-toc-3.0.0.zip", [
    ("RootToc.toc", toc("Root TOC", "3.0.0", interface="120105")),
    ("RootToc_Vanilla.toc", toc("Root TOC", "3.0.0", interface="11509")),
    ("core.lua", "-- RootToc 3.0.0\n"),
])

write("wrapped-4.0.0.zip", [
    ("wrapped-4.0.0/Wrapped/Wrapped.toc", toc("Wrapped", "4.0.0")),
    ("wrapped-4.0.0/Wrapped/core.lua", "-- Wrapped 4.0.0\n"),
])

write("no-version.zip", [
    ("NoVersion/NoVersion.toc", "## Interface: 16001\r\n## Title: No Version\r\n\r\ncore.lua\r\n"),
    ("NoVersion/core.lua", "-- no ## Version line\n"),
])

write("stale-interface-1.0.0.zip", [
    ("StaleInterface/StaleInterface.toc", toc("Stale Interface", "1.0.0", interface="110207")),
    ("StaleInterface/core.lua", "-- an interface of another major than the fixture builds (task 015)\n"),
])

write("escaping-path.zip", [
    ("Escape/Escape.toc", toc("Escape", "0.1.0")),
    ("Escape/../../Outside.lua", "-- must never be written\n"),
])

write("needs-library-1.0.0.zip", [
    ("NeedsLibrary/NeedsLibrary.toc",
     toc("Needs Library", "1.0.0",
         extra="## Dependencies: LibEmbedded, LibMissing, Blizzard_Something\r\n## OptionalDeps: LibOptional\r\n")),
    ("NeedsLibrary/core.lua", "-- NeedsLibrary 1.0.0\n"),
    ("NeedsLibrary/Libs/LibEmbedded/LibEmbedded.lua", "-- an embedded library\n"),
])

write("no-addon.zip", [
    ("readme.txt", "no addon folder anywhere\n"),
    ("Docs/notes.txt", "still no TOC\n"),
])

# The forever asset of `github:Questie/Questie` v1.2.3, at the URL path the folder stub maps its download to.
# The TOC keeps the tag's `v`, so a test sees the sidecar record the pick's version (`1.2.3`) instead.
write(RELEASES / "github.com/Questie/Questie/releases/download/v1.2.3/Questie-v1.2.3-forever.zip", [
    ("Questie/", ""),
    ("Questie/Questie.toc", toc("Questie", "v1.2.3", extra="## Notes: a made-up stand-in, no real addon\r\n")),
    ("Questie/core.lua", "-- Questie fixture 1.2.3\n"),
])

# Task 039, the bound on the unpacked size. A zip that declares 3 GiB in a few hundred bytes: refused by the
# bound (2 GiB) before anything is written, installed under a higher one (its data is a few bytes).
write("declares-too-much-1.0.0.zip", [
    ("DeclaresTooMuch/DeclaresTooMuch.toc", toc("Declares Too Much", "1.0.0")),
    ("DeclaresTooMuch/core.lua", "-- DeclaresTooMuch 1.0.0\n"),
])
declare_size("declares-too-much-1.0.0.zip", "DeclaresTooMuch/core.lua", 3 * 1024 ** 3)

# A zip that declares 16 bytes for a file that unpacks to 66 000: refused while it is written.
write("lies-about-its-size-1.0.0.zip", [
    ("LiesAboutItsSize/LiesAboutItsSize.toc", toc("Lies About Its Size", "1.0.0")),
    ("LiesAboutItsSize/core.lua", "-- padding\n" * 6000),
])
declare_size("lies-about-its-size-1.0.0.zip", "LiesAboutItsSize/core.lua", 16)

# The same two shapes as WTF backups, for the restore.
write("wtf-declares-too-much.zip", [
    ("Config.wtf", 'SET portal "EU"\r\n'),
])
declare_size("wtf-declares-too-much.zip", "Config.wtf", 3 * 1024 ** 3)

write("wtf-lies-about-its-size.zip", [
    ("Config.wtf", 'SET portal "EU"\r\n' * 4000),
])
declare_size("wtf-lies-about-its-size.zip", "Config.wtf", 16)
