"""Writes CHANGELOG.md from the done task files and prints the notes of one release.

The task files are the history (tasks/README.md). Every file under tasks/done/ is one line of the changelog: its
H1 without the id, linked to the task file, under the milestone of its frontmatter
in roadmap order ("Other" for a task without one). A task belongs to the first `v<major>.<minor>.<patch>` tag
whose tree holds it under tasks/done/; a task no tag holds yet is Unreleased. The tag trees decide, not the
`closed` date: on the day of a tag, tasks close both before and after it.

A task whose frontmatter says `changelog: housekeeping` changed the project rather than the app (the repo, CI,
the process); it is listed last, under "Project housekeeping", and left out of the release notes, which only
count it and point to CHANGELOG.md.

A section's summary, the prose between its `## ` heading and the generated list, is written by hand and kept by
every run; when a new tag appears, the Unreleased summary moves to that tag's section. So before a release, write
the summary under Unreleased; after the tag, a run stamps the section with the version and the tag's date.

    python -I scripts/changelog.py                     # rewrite CHANGELOG.md
    python -I scripts/changelog.py --check             # exit 1 when CHANGELOG.md is out of date
    python -I scripts/changelog.py --notes 0.2.0 --out dist/release-notes-0.2.0.md

Between a tag push and the commit that stamps its section, `--check` reads a newest tag that CHANGELOG.md has no
`## <version>` heading for as not there yet, so its tasks are still checked under Unreleased and a pull request
opened in that window stays green; a run without `--check` stamps it.

`--notes` prints (or writes) one section as release notes, with a footer on the checksum and the license file;
scripts/release.ps1 calls it and the release job publishes the result. A version without a tag yet gets the
Unreleased section (a dry run before the tag). Needs git with the tags (CI: a full-depth checkout) and nothing
beyond the standard library; the frontmatter reader and the roadmap are the ones of scripts/sync-issues.py.
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHANGELOG = ROOT / "CHANGELOG.md"
UNRELEASED = "Unreleased"
RELEASE_TAG = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
VERSION = re.compile(r"^\d+\.\d+\.\d+$")
MARKER = "<!-- The summary above is written by hand; the list below is generated from tasks/done/ by scripts/changelog.py. -->"
OTHER = "Other"
HOUSEKEEPING = "Project housekeeping"


def _load_sync_issues():
    """scripts/sync-issues.py as a module: `python -I` keeps the script's folder off sys.path, and the hyphen
    keeps it from being imported by name."""
    spec = importlib.util.spec_from_file_location("sync_issues", Path(__file__).with_name("sync-issues.py"))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module  # its dataclasses look their module up there
    spec.loader.exec_module(module)
    return module


sync_issues = _load_sync_issues()


# --- the inputs -----------------------------------------------------------------------------


@dataclass
class Task:
    key: str  # "021"
    name: str  # "021-slug.md"
    title: str  # the H1 without the id
    milestone: str | None
    housekeeping: bool = False


@dataclass
class Section:
    version: str  # "0.1.0" | UNRELEASED
    date: str | None  # the tag's date
    tasks: list[Task] = field(default_factory=list)
    summary: str = ""


def git(*args: str) -> str:
    result = subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, text=True, encoding="utf-8")
    if result.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed ({result.returncode}): {result.stderr.strip()}")
    return result.stdout


def task_key(name: str) -> str | None:
    match = sync_issues.TASK_FILE.match(name)
    return None if match is None else match.group(1) + match.group(2)


def load_done() -> dict[str, Task]:
    tasks: dict[str, Task] = {}
    for path in sorted((sync_issues.TASKS / "done").glob("*.md")):
        key = task_key(path.name)
        if key is None:
            raise SystemExit(f"done/{path.name}: not a task file name")
        rel = f"done/{path.name}"
        data, body = sync_issues.parse_frontmatter(path.read_text(encoding="utf-8"), rel)
        heading = sync_issues._heading(body, rel)
        title = re.sub(r"^\d{2,4}[a-z]?\s*[—–-]\s*", "", heading)
        kind = sync_issues.scalar(data, "changelog")
        if kind not in (None, "housekeeping"):
            raise SystemExit(f"{rel}: `changelog: {kind}` is not known (only `housekeeping`)")
        tasks[key] = Task(
            key=key,
            name=path.name,
            title=title,
            milestone=sync_issues.scalar(data, "milestone"),
            housekeeping=kind == "housekeeping",
        )
    return tasks


def release_tags() -> list[tuple[str, str]]:
    """(version, date) of every release tag, oldest first."""
    out = git("for-each-ref", "--format=%(refname:short)%09%(creatordate:short)", "refs/tags/v*")
    tags: list[tuple[tuple[int, int, int], str, str]] = []
    for line in out.splitlines():
        name, _, date = line.partition("\t")
        match = RELEASE_TAG.match(name)
        if match:
            tags.append(((int(match.group(1)), int(match.group(2)), int(match.group(3))), name[1:], date))
    return [(version, date) for _, version, date in sorted(tags)]


def keys_at(version: str) -> set[str]:
    names = git("ls-tree", "--name-only", f"v{version}", "--", "tasks/done/").splitlines()
    return {key for key in (task_key(Path(n).name) for n in names) if key is not None}


def repo_url() -> str:
    cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^repository = "([^"]+)"', cargo)
    if match is None:
        raise SystemExit("Cargo.toml: no `repository` in [workspace.package]")
    return match.group(1).rstrip("/")


def read_summaries(text: str) -> dict[str, str]:
    """version → the hand-written prose of its section in an existing CHANGELOG.md."""
    bodies: dict[str, list[str]] = {}
    current: list[str] | None = None
    for line in text.replace("\r\n", "\n").splitlines():
        heading = re.match(r"^##\s+(\S+)", line)
        if heading:
            current = bodies.setdefault(heading.group(1), [])
        elif current is not None:
            current.append(line)
    summaries: dict[str, str] = {}
    for version, lines in bodies.items():
        # Up to the marker; a section written without one ends its summary at the first `### ` list heading.
        stripped = [line.strip() for line in lines]
        if MARKER in stripped:
            end = stripped.index(MARKER)
        else:
            end = next((i for i, line in enumerate(lines) if line.startswith("### ")), len(lines))
        summaries[version] = sync_issues._trim(lines[:end])
    return summaries


# --- the sections ---------------------------------------------------------------------------


def unstamped_tags(tags: list[tuple[str, str]], summaries: dict[str, str]) -> tuple[list[tuple[str, str]], list[str]]:
    """The tags without the newest ones CHANGELOG.md has no section for yet, and those versions. Only the newest
    run counts: an older tag without its section is still an error for `--check`."""
    kept = list(tags)
    pending: list[str] = []
    while kept and kept[-1][0] not in summaries:
        pending.insert(0, kept.pop()[0])
    return kept, pending


def build_sections(tags: list[tuple[str, str]]) -> list[Section]:
    """Newest first: Unreleased, then every release tag of `tags` (oldest first)."""
    done = load_done()
    claimed: set[str] = set()
    sections: list[Section] = []
    for version, date in tags:
        keys = keys_at(version) - claimed
        claimed |= keys
        missing = sorted(keys - done.keys())
        if missing:
            print(f"warning: v{version} holds {', '.join(missing)} under tasks/done/, the working tree does not", file=sys.stderr)
        sections.append(Section(version=version, date=date, tasks=[done[k] for k in sorted(keys) if k in done]))
    unreleased = Section(version=UNRELEASED, date=None, tasks=[done[k] for k in sorted(done.keys() - claimed)])
    sections.reverse()
    return [unreleased, *sections]


def attach_summaries(sections: list[Section], summaries: dict[str, str]) -> None:
    """Keeps every section's summary; the newest tag without a section yet takes the Unreleased one."""
    unreleased = summaries.get(UNRELEASED, "")
    for section in sections[1:]:
        if section.version in summaries:
            section.summary = summaries[section.version]
        elif unreleased:
            section.summary, unreleased = unreleased, ""
    sections[0].summary = unreleased


def task_lines(section: Section, url: str, housekeeping: bool = True) -> list[str]:
    """The milestone groups in roadmap order, "Other", then "Project housekeeping" (left out with
    `housekeeping=False`)."""
    order = list(sync_issues.load_roadmap())
    groups: dict[str, list[Task]] = {}
    chores: list[Task] = []
    for task in section.tasks:
        if task.housekeeping:
            chores.append(task)
        else:
            groups.setdefault(task.milestone or OTHER, []).append(task)
    named = sorted(groups, key=lambda m: (m == OTHER, order.index(m) if m in order else len(order), m))
    if housekeeping and chores:
        named.append(HOUSEKEEPING)
        groups[HOUSEKEEPING] = chores
    lines: list[str] = []
    for name in named:
        lines += [f"### {name}", ""]
        for task in groups[name]:
            lines.append(f"- {task.title} ([{task.key}]({url}/blob/main/tasks/done/{task.name}))")
        lines.append("")
    if not lines:
        lines = ["No task closed yet.", ""]
    return lines


def section_body(section: Section, url: str, marker: bool = True, housekeeping: bool = True) -> list[str]:
    lines: list[str] = []
    if section.summary:
        lines += [section.summary, ""]
    if marker:
        lines += [MARKER, ""]
    return lines + task_lines(section, url, housekeeping)


def heading(section: Section) -> str:
    return f"## {section.version}" + (f" — {section.date}" if section.date else "")


def render_changelog(sections: list[Section], url: str) -> str:
    lines = [
        "# Changelog",
        "",
        "Every release of mrgl and the tasks it closed, newest first. The task lists are generated by",
        "`scripts/changelog.py` from the task files under `tasks/done/` and the release tags: edit the task file,",
        "not the list. The summary under a heading is written by hand and kept by the generator. Each release's zip",
        f"(with its third-party licenses) and its `SHA256SUMS.txt` are on the [releases page]({url}/releases).",
        "",
    ]
    for section in sections:
        lines += [heading(section), ""] + section_body(section, url)
    return "\n".join(lines).rstrip("\n") + "\n"


def render_notes(sections: list[Section], version: str, url: str) -> str:
    tagged = {s.version: s for s in sections}
    section = tagged.get(version) or Section(version=version, date=None, tasks=sections[0].tasks, summary=sections[0].summary)
    zip_name = f"mrgl-{version}.zip"
    chores = sum(task.housekeeping for task in section.tasks)
    lines = section_body(section, url, marker=False, housekeeping=False)
    if chores:
        listed = "task (repository, CI or process) is" if chores == 1 else "tasks (repository, CI and process) are"
        lines += [
            f"{chores} project housekeeping {listed} listed in [CHANGELOG.md]({url}/blob/main/CHANGELOG.md).",
            "",
        ]
    lines += [
        "### Verify the download",
        "",
        f"`{zip_name}` holds the Windows exes. `SHA256SUMS.txt` holds the zip's SHA-256; in PowerShell,",
        f"`(Get-FileHash {zip_name}).Hash` prints the same value in capitals. The zip also has a build provenance",
        f"attestation: `gh attestation verify {zip_name} --repo speti4/mrgl` shows that this repository's release",
        "workflow built it. The zip holds `THIRD-PARTY-LICENSES.html`, the open-source crates compiled into the exes",
        f"and their licenses. The full history is in [CHANGELOG.md]({url}/blob/main/CHANGELOG.md).",
    ]
    return sync_issues._trim(lines) + "\n"


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # titles carry em dashes; Windows consoles may not
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="exit 1 when CHANGELOG.md is out of date, write nothing")
    mode.add_argument("--notes", metavar="VERSION", help="the release notes of VERSION (e.g. 0.2.0) instead of CHANGELOG.md")
    parser.add_argument("--out", type=Path, help="with --notes: write the notes to this file instead of stdout")
    args = parser.parse_args()
    if args.out is not None and args.notes is None:
        parser.error("--out goes with --notes")

    url = repo_url()
    current = CHANGELOG.read_text(encoding="utf-8") if CHANGELOG.is_file() else ""
    summaries = read_summaries(current)
    tags = release_tags()
    if args.check:
        tags, pending = unstamped_tags(tags, summaries)
        notice = "::notice::" if os.environ.get("GITHUB_ACTIONS") == "true" else "notice: "
        for version in pending:
            print(f"{notice}v{version} has no section in CHANGELOG.md yet; its tasks are checked under Unreleased"
                  " until `python -I scripts/changelog.py` stamps it")
    sections = build_sections(tags)
    attach_summaries(sections, summaries)

    if args.notes is not None:
        version = args.notes.removeprefix("v")
        if not VERSION.match(version):
            parser.error(f"--notes takes a version like 0.2.0, not {args.notes!r}")
        notes = render_notes(sections, version, url)
        if args.out is None:
            sys.stdout.write(notes)
        else:
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(notes, encoding="utf-8", newline="\n")
        return 0

    text = render_changelog(sections, url)
    if current.replace("\r\n", "\n") == text:
        print("CHANGELOG.md is up to date")
        return 0
    if args.check:
        print("CHANGELOG.md is out of date: run `python -I scripts/changelog.py`")
        return 1
    CHANGELOG.write_text(text, encoding="utf-8", newline="\n")
    counts = ", ".join(f"{s.version} {len(s.tasks)}" for s in sections)
    print(f"CHANGELOG.md written ({counts} tasks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
