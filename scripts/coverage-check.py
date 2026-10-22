"""Prints the coverage table of a cargo-llvm-cov lcov export and fails below the floors.

The `coverage` job (.github/workflows/ci.yml) and the local run of CONTRIBUTING.md measure the three crates with
cargo-llvm-cov on the nightly of scripts/coverage-toolchain.txt, export the result as lcov and hand it here:

    cargo +nightly-YYYY-MM-DD llvm-cov --workspace --branch --locked --lcov --output-path lcov.info
    python -I scripts/coverage-check.py lcov.info

The lcov export is read, not the JSON summary: the summary counts every instantiation of a generic function on
its own, so a branch one instantiation never takes counts as missed while the text and HTML reports, which merge
the instantiations, show it covered. The lcov lines (`DA`) and branches (`BRDA`) are the merged view, the one a
reader of the report can act on.

The table (Markdown, so the job summary renders it) has one row per crate and one per file with something
missed, sorted by missed lines: lines covered with the missed count, branches covered with the missed count. The
run fails when the workspace total of either is below its floor, the two constants below: they are the one
place the floors are written, they only rise (every pull request that lands tests raises them to what it
measured), and they end at 100 when task 117 closes.

Two kinds of exclusion are checked against the code. An item a test cannot reach carries
`#[cfg_attr(coverage_nightly, coverage(off))]` under one comment line `// coverage: off — <reason>` (more
comment lines continue the reason); a `#[cfg(test)]` item carries the attribute with no reason (the test code is
not the subject). docs/development.md, "Coverage", lists every reasoned site as `- `<path under crates/>`:
<reason>`, the same words; the check fails on an attribute with neither a `#[cfg(test)]` nor a reason above it,
and on any difference between the sites in the code and the list in the docs. A failure arm only the operating
system can provoke (a read of a folder the code just listed, a write to a file it just created, a system call's
own failure) carries `// coverage: os` at the end of its line, or sits between a `// coverage: os-begin` line and
a `// coverage: os-end` line; those lines and their branches are left out of the counts, and a marked line a test
does reach fails the run, so a stale mark cannot hide a line that became testable. Nothing beyond the standard
library.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict
from pathlib import Path

LINES_FLOOR = 94.5
BRANCHES_FLOOR = 95.5

ATTRIBUTE = "#[cfg_attr(coverage_nightly, coverage(off))]"
REASON_PREFIX = "// coverage: off — "
OS_LINE = re.compile(r"//\s*coverage: os(\s|$|—)")
OS_BEGIN = re.compile(r"//\s*coverage: os-begin(\s|$|—)")
OS_END = re.compile(r"//\s*coverage: os-end(\s|$|—)")
DOCS = Path("docs/development.md")
DOCS_HEADING = "### Coverage"


def short(path: str) -> str:
    """`mrgl-core/src/x.rs` from any spelling of the full path, or the path itself outside `crates/`."""
    parts = path.replace("\\", "/").split("/")
    return "/".join(parts[parts.index("crates") + 1 :]) if "crates" in parts else path


def read_lcov(path: Path) -> dict[str, tuple[dict[int, int], dict[tuple[int, str, str], bool]]]:
    """Per file: the hit count per line (the most of any instantiation) and whether each branch was taken."""
    files: dict[str, tuple[dict[int, int], dict[tuple[int, str, str], bool]]] = {}
    current = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("SF:"):
            current = files.setdefault(short(line[3:]), ({}, {}))
        elif line.startswith("DA:") and current:
            number, hits = line[3:].split(",")[:2]
            current[0][int(number)] = max(current[0].get(int(number), 0), int(hits))
        elif line.startswith("BRDA:") and current:
            number, block, branch, taken = line[5:].split(",")
            key = (int(number), block, branch)
            current[1][key] = current[1].get(key, False) or taken not in ("-", "0")
    return files


def os_marks(root: Path) -> dict[str, set[int]]:
    """The lines marked as reachable only through the operating system, per file under `crates/`."""
    marks: dict[str, set[int]] = defaultdict(set)
    for path in sorted((root / "crates").glob("*/src/**/*.rs")):
        name = "/".join(path.relative_to(root / "crates").parts)
        inside = False
        for number, text in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if OS_BEGIN.search(text):
                inside = True
            elif OS_END.search(text):
                inside = False
            elif inside or OS_LINE.search(text):
                marks[name].add(number)
    return marks


class Count:
    def __init__(self) -> None:
        self.lines = 0
        self.lines_missed = 0
        self.branches = 0
        self.branches_missed = 0

    def add(self, other: "Count") -> None:
        self.lines += other.lines
        self.lines_missed += other.lines_missed
        self.branches += other.branches
        self.branches_missed += other.branches_missed

    def percent(self, kind: str) -> float:
        total = getattr(self, kind)
        return 100.0 if total == 0 else (total - getattr(self, f"{kind}_missed")) / total * 100

    def cell(self, kind: str) -> str:
        total = getattr(self, kind)
        if total == 0:
            return "–"
        return f"{self.percent(kind):.2f}% ({getattr(self, f'{kind}_missed')} missed of {total})"


def count(lines: dict[int, int], branches: dict[tuple[int, str, str], bool], marked: set[int]) -> Count:
    result = Count()
    for number, hits in lines.items():
        if number in marked:
            continue
        result.lines += 1
        result.lines_missed += hits == 0
    for (number, _, _), taken in branches.items():
        if number in marked:
            continue
        result.branches += 1
        result.branches_missed += not taken
    return result


def sites_in_code(root: Path) -> tuple[set[tuple[str, str]], list[str]]:
    """The reasoned exclusions of the crates as (path, reason), and the attributes with no reason."""
    sites: set[tuple[str, str]] = set()
    problems: list[str] = []
    for path in sorted((root / "crates").glob("*/src/**/*.rs")):
        lines = path.read_text(encoding="utf-8").split("\n")
        name = "/".join(path.relative_to(root / "crates").parts)
        for index, line in enumerate(lines):
            if line.strip() != ATTRIBUTE:
                continue
            # Upwards from the attribute: over the other attributes, then the comment lines; the reason starts at
            # the `// coverage: off —` line. A `#[cfg(test)]` among the attributes is the test class, no reason.
            above = index - 1
            attributes: list[str] = []
            while above >= 0 and lines[above].strip().startswith("#["):
                attributes.append(lines[above].strip())
                above -= 1
            if "#[cfg(test)]" in attributes:
                continue
            comment: list[str] = []
            while above >= 0 and lines[above].strip().startswith("//"):
                comment.insert(0, lines[above].strip())
                above -= 1
            try:
                first = next(i for i, text in enumerate(comment) if text.startswith(REASON_PREFIX))
            except StopIteration:
                problems.append(
                    f"{name}:{index + 1}: a coverage(off) attribute without a `{REASON_PREFIX.strip()}` line above it"
                )
                continue
            reason = [comment[first][len(REASON_PREFIX) :]] + [text[3:] for text in comment[first + 1 :]]
            sites.add((name, " ".join(" ".join(reason).split())))
    return sites, problems


def sites_in_docs(root: Path) -> set[tuple[str, str]]:
    text = (root / DOCS).read_text(encoding="utf-8")
    start = text.find(DOCS_HEADING)
    if start < 0:
        return set()
    section = text[start + len(DOCS_HEADING) :]
    end = re.search(r"^#{2,3} ", section, re.MULTILINE)
    section = section[: end.start()] if end else section
    # A bullet may wrap onto indented continuation lines.
    bullets = re.findall(r"^- `([^`]+)`: (.+?)(?=^\S|\Z)", section, re.MULTILINE | re.DOTALL)
    return {(path, " ".join(reason.split())) for path, reason in bullets}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("lcov", type=Path, help="the --lcov export of cargo-llvm-cov")
    parser.add_argument("--repo", type=Path, default=Path("."), help="the repo root (default: the current folder)")
    args = parser.parse_args()

    files = read_lcov(args.lcov)
    marks = os_marks(args.repo)
    failures: list[str] = []
    total = Count()
    crates: dict[str, Count] = defaultdict(Count)
    rows: list[tuple[str, Count]] = []
    marked_lines = marked_branches = 0
    for name, (lines, branches) in sorted(files.items()):
        marked = marks.get(name, set())
        counted = count(lines, branches, marked)
        total.add(counted)
        crates[name.split("/")[0]].add(counted)
        rows.append((name, counted))
        for number in sorted(marked):
            if number in lines:
                marked_lines += 1
            marked_branches += sum(1 for (line, _, _) in branches if line == number)
            if lines.get(number, 0) > 0 and all(taken for (line, _, _), taken in branches.items() if line == number):
                failures.append(f"{name}:{number} is marked `coverage: os` but a test reaches it: drop the mark")

    out = [
        f"Coverage: {total.cell('lines')} lines, {total.cell('branches')} branches; "
        f"floors {LINES_FLOOR} / {BRANCHES_FLOOR}; "
        f"marked `coverage: os`: {marked_lines} lines and {marked_branches} branches left out",
        "",
        "| Crate or file | Lines | Branches |",
        "|---|---|---|",
    ]
    for crate in sorted(crates):
        out.append(f"| **{crate}** | {crates[crate].cell('lines')} | {crates[crate].cell('branches')} |")
    for name, counted in sorted(rows, key=lambda row: (-row[1].lines_missed, row[0])):
        if counted.lines_missed or counted.branches_missed:
            out.append(f"| `{name}` | {counted.cell('lines')} | {counted.cell('branches')} |")
    print("\n".join(out))

    if total.percent("lines") < LINES_FLOOR:
        failures.append(f"lines {total.percent('lines'):.2f}% are below the floor of {LINES_FLOOR}%")
    if total.percent("branches") < BRANCHES_FLOOR:
        failures.append(f"branches {total.percent('branches'):.2f}% are below the floor of {BRANCHES_FLOOR}%")

    code, problems = sites_in_code(args.repo)
    docs = sites_in_docs(args.repo)
    failures.extend(problems)
    for path, reason in sorted(code - docs):
        failures.append(f"excluded in the code, not in the docs list: `{path}`: {reason}")
    for path, reason in sorted(docs - code):
        failures.append(f"in the docs list, not in the code: `{path}`: {reason}")
    print(f"\nExclusions: {len(code)} reasoned site(s) in the code, {len(docs)} in the docs list")

    for failure in failures:
        print(f"::error::{failure}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
