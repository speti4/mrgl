"""Writes the synthetic fixture zips of task 004 next to this script, deterministically (fixed timestamps,
fixed entry order, deflate), so a rerun changes no byte. Run it as `python -I <this file>` after editing a
shape below; the zips are committed. Nothing here comes from a real addon."""

from pathlib import Path
import zipfile

HERE = Path(__file__).resolve().parent
EPOCH = (1980, 1, 1, 0, 0, 0)


def toc(name, version, interface="16001", extra=""):
    return (
        f"## Interface: {interface}\r\n## Title: {name}\r\n## Version: {version}\r\n"
        f"## Author: mrgl fixtures\r\n{extra}\r\ncore.lua\r\n"
    )


def write(file_name, entries):
    path = HERE / file_name
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in entries:
            info = zipfile.ZipInfo(name, date_time=EPOCH)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o644 << 16) if not name.endswith("/") else (0o755 << 16) | 0x10
            archive.writestr(info, content)
    print(path.name, [name for name, _ in entries])


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

write("escaping-path.zip", [
    ("Escape/Escape.toc", toc("Escape", "0.1.0")),
    ("Escape/../../Outside.lua", "-- must never be written\n"),
])

write("needs-library-1.0.0.zip", [
    ("NeedsLibrary/NeedsLibrary.toc",
     toc("Needs Library", "1.0.0",
         extra="## Dependencies: LibEmbedded, LibMissing\r\n## OptionalDeps: LibOptional\r\n")),
    ("NeedsLibrary/core.lua", "-- NeedsLibrary 1.0.0\n"),
    ("NeedsLibrary/Libs/LibEmbedded/LibEmbedded.lua", "-- an embedded library\n"),
])

write("no-addon.zip", [
    ("readme.txt", "no addon folder anywhere\n"),
    ("Docs/notes.txt", "still no TOC\n"),
])
