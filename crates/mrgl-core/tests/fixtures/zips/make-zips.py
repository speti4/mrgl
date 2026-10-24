"""Writes the synthetic fixture zips of task 004 next to this script, the release asset the GitHub source's
folder stub serves (task 040) under `../releases/`, and the files the WoWInterface stub serves (task 094) under
`../sources/`, deterministically (fixed timestamps, fixed entry order, deflate; the WoWInterface files are
stored, not deflated, so their bytes and the MD5 their answers name never depend on the zlib at hand), so a
rerun changes no byte. Run it as `python -I <this file>` after editing a shape below; the zips are committed.
Nothing here comes from a real addon."""

import calendar
from pathlib import Path
import struct
import zipfile

HERE = Path(__file__).resolve().parent
RELEASES = HERE.parent / "releases"
SOURCES = HERE.parent / "sources"
EPOCH = (1980, 1, 1, 0, 0, 0)


def toc(name, version, interface="16001", extra=""):
    return (
        f"## Interface: {interface}\r\n## Title: {name}\r\n## Version: {version}\r\n"
        f"## Author: mrgl fixtures\r\n{extra}\r\ncore.lua\r\n"
    )


def write(file_name, entries, stored=False):
    path = HERE / file_name
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in entries:
            info = zipfile.ZipInfo(name, date_time=EPOCH)
            info.compress_type = zipfile.ZIP_STORED if stored else zipfile.ZIP_DEFLATED
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

# The shape of a boss-mod core zip (task 065): no folder is extended by all the others, and the first by name
# is not the core; the zip's stem names it.
write("boss-core-5.0.0.zip", [
    ("Boss-Arena/Boss-Arena.toc", toc("Boss: Arena", "5.0.0", extra="## Dependencies: Boss-Core\r\n")),
    ("Boss-Arena/core.lua", "-- Boss-Arena 5.0.0\n"),
    ("Boss-Core/Boss-Core.toc", toc("Boss Core", "5.0.0")),
    ("Boss-Core/core.lua", "-- Boss-Core 5.0.0\n"),
    ("Boss-GUI/Boss-GUI.toc", toc("Boss: GUI", "5.0.0", extra="## Dependencies: Boss-Core\r\n")),
    ("Boss-GUI/core.lua", "-- Boss-GUI 5.0.0\n"),
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

# The Forever fixture build is 1.60.1 (interface 16001); 16000 is of its expansion and behind it: the client's
# `Out of date` (tasks 076, 082).
write("behind-interface-1.0.0.zip", [
    ("BehindInterface/BehindInterface.toc", toc("Behind Interface", "1.0.0", interface="16000")),
    ("BehindInterface/core.lua", "-- an interface one patch behind the Forever fixture build (task 082)\n"),
])

write("escaping-path.zip", [
    ("Escape/Escape.toc", toc("Escape", "0.1.0")),
    ("Escape/../../Outside.lua", "-- must never be written\n"),
])

# Task 124 (finding L6 of task 121): a folder ending in a dot next to its twin without one; Windows drops the
# dot, so both would unpack into one folder and the file list the sidecar records would name paths that are
# not on disk. The entry rule refuses the zip.
write("trailing-dot-entry.zip", [
    ("TrailingDot/TrailingDot.toc", toc("Trailing Dot", "1.0.0")),
    ("TrailingDot/core.lua", "-- the folder as written\n"),
    ("TrailingDot./core.lua", "-- the same folder once Windows drops the dot\n"),
])

# Task 123 (finding H1 of task 121): a TOC at the root whose stem is `.`, `..` or a blank, names the entry
# check lets through since none of their components is `.` or `..`. The install falls back to the zip's stem;
# a test that copies one under a stem that is no folder name either (`..zip`) sees the zip refused.
for file_name, toc_name in [
    ("dot-toc.zip", "..toc"),
    ("dot-dot-toc.zip", "...toc"),
    ("blank-toc.zip", " .toc"),
]:
    write(file_name, [
        (toc_name, toc("Odd Root TOC", "1.0.0")),
        ("core.lua", "-- a root TOC whose stem is no folder name\n"),
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

# The files behind the WoWInterface fixture projects (task 094), at the path and query the folder stub maps
# their `UIDownload` to (`getfile.php@d=<date>&id=<id>&minion`). Their TOCs name an interface of every fixture
# build, so the interface check stays quiet whichever flavor a test installs them into.
WOWI = SOURCES / "cdn.wowinterface.com/downloads"
EVERY_BUILD = "120105, 11507, 40402, 16001"
for project, folder, version in [
    (1001, "MrglCompat", "2.0.0"),
    (1002, "MrglNoList", "1.0.0"),
    (1004, "MrglBadSum", "1.0.0"),
    (1005, "MrglForever", "3.1.0"),
]:
    write(WOWI / f"getfile.php@d=1785000000&id={project}&minion", [
        (f"{folder}/", ""),
        (f"{folder}/{folder}.toc", toc(folder, version, interface=EVERY_BUILD,
                                       extra=f"## X-WoWI-ID: {project}\r\n")),
        (f"{folder}/core.lua", f"-- {folder} fixture {version}\n"),
    ], stored=True)

# The files behind the `download_link` of the Wago fixture addon `MrglWg01` (task 095): one per game version,
# each with the TOC of its flavor, so a test sees the Forever and the retail answer of one URL install apart.
WAGO = SOURCES / "addons.wago.io/downloads/MrglWg01"
for game, interface in [("forever", "16001"), ("retail", "120105")]:
    write(WAGO / f"2.0.0-{game}.zip", [
        ("MrglWagoOne/", ""),
        ("MrglWagoOne/MrglWagoOne.toc", toc("MrglWagoOne", "2.0.0", interface=interface,
                                            extra="## X-Wago-ID: MrglWg01\r\n")),
        ("MrglWagoOne/core.lua", f"-- MrglWagoOne fixture 2.0.0 for {game}\n"),
    ], stored=True)

# The files behind the `downloadUrl` of the CurseForge fixture project 4242 (task 096): the Forever file on
# the CDN, the retail one where the CDN's redirect points; and the zip in hand of project 4244, whose author
# allows no download outside CurseForge (it stays next to this script, as a zip the user downloaded).
CURSE = SOURCES / "edge.forgecdn.net/files/7001/1"
CURSE_MEDIA = SOURCES / "mediafilez.forgecdn.net/files/7002/2"
for path, interface, game in [
    (CURSE / "MrglCurseOnly-1.0.0.zip", "16001", "forever"),
    (CURSE_MEDIA / "MrglCurseOnly-1.0.0-retail.zip", "120105", "retail"),
]:
    write(path, [
        ("MrglCurseOnly/", ""),
        ("MrglCurseOnly/MrglCurseOnly.toc", toc("MrglCurseOnly", "1.0.0", interface=interface,
                                                extra="## X-Curse-Project-ID: 4242\r\n")),
        ("MrglCurseOnly/core.lua", f"-- MrglCurseOnly fixture 1.0.0 for {game}\n"),
    ], stored=True)
write("MrglNoDistribution-2.0.0.zip", [
    ("MrglNoDistribution/", ""),
    ("MrglNoDistribution/MrglNoDistribution.toc", toc("MrglNoDistribution", "2.0.0",
                                                      extra="## X-Curse-Project-ID: 4244\r\n")),
    ("MrglNoDistribution/core.lua", "-- MrglNoDistribution fixture 2.0.0\n"),
], stored=True)

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


def filetime(year, month, day, hour, minute, second):
    """A Windows file time of a UTC moment: 100-nanosecond intervals since 1601-01-01."""
    unix = calendar.timegm((year, month, day, hour, minute, second))
    return (unix + 11_644_473_600) * 10_000_000


# Task 117: the three shapes of an entry's date a restore reads when the extended timestamp field (0x5455)
# a backup writes is missing: no DOS date at all (zero date fields, which read as no time), the default
# 1980-01-01 00:00:00 (what every zip above carries), and a real DOS date next to an NTFS extra field
# (0x000a), which the restore passes over for the zip's own fields read as local time.
NTFS_FIELD = struct.pack("<HHIHHQQQ", 0x000A, 32, 0, 1, 24, *[filetime(2024, 3, 1, 12, 30, 10)] * 3)
path = HERE / "wtf-dos-times.zip"
with zipfile.ZipFile(path, "w") as archive:
    for name, date_time, extra in [
        ("Config.wtf", (1980, 0, 0, 0, 0, 0), b""),
        ("Account/MRGLFIXTURE/config-cache.wtf", EPOCH, b""),
        ("Account/MRGLFIXTURE/bindings-cache.wtf", (2024, 3, 1, 12, 30, 10), NTFS_FIELD),
    ]:
        info = zipfile.ZipInfo(name, date_time=date_time)
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o644 << 16
        info.extra = extra
        archive.writestr(info, f'SET name "{name}"\r\n')
print(path.name, "(no date, the default date, a dated entry with an NTFS field)")
