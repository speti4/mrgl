"""Prints the coverage table of a cargo-llvm-cov summary and fails below the floors.

The `coverage` job (.github/workflows/ci.yml) and the local run of CONTRIBUTING.md measure the three crates with
cargo-llvm-cov on the nightly of scripts/coverage-toolchain.txt, export the summary as JSON and hand it here:

    cargo +nightly-YYYY-MM-DD llvm-cov --workspace --branch --locked --json --summary-only --output-path summary.json
    python -I scripts/coverage-check.py summary.json

The table (Markdown, so the job summary renders it) has one row per crate and one per file, sorted by missed
lines: lines covered with the missed count, branches covered with the missed count. The run fails when the
workspace total of either is below its floor, the two constants below: they are the one place the floors are
written, they only rise (every pull request that lands tests raises them to what it measured), and they end at
100 when task 117 closes.

The second check is the exclusion register. An item a test cannot reach carries `#[cfg_attr(coverage_nightly,
coverage(off))]` under one comment line `// coverage: off — <reason>` (more comment lines continue the reason);
a `#[cfg(test)]` item carries the attribute with no reason (the test code is not the subject). docs/development.md,
"Coverage", lists every reasoned site as `- `<path under crates/>`: <reason>`, the same words. The check fails
on an attribute with neither a `#[cfg(test)]` nor a reason above it, and on any difference between the sites
in the code and the list in the docs, so the list cannot go stale. Nothing beyond the standard library.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

LINES_FLOOR = 86.0
BRANCHES_FLOOR = 81.0

ATTRIBUTE = "#[cfg_attr(coverage_nightly, coverage(off))]"
REASON_PREFIX = "// coverage: off — "
DOCS = Path("docs/development.md")
DOCS_HEADING = "### Coverage"


def crate_of(path: str) -> str:
    """`mrgl-core` from any spelling of `…/crates/mrgl-core/src/x.rs`."""
    parts = path.replace("\\", "/").split("/")
    return parts[parts.index("crates") + 1] if "crates" in parts else "?"


def short(path: str) -> str:
    """`mrgl-core/src/x.rs` from any spelling of the full path."""
    parts = path.replace("\\", "/").split("/")
    return "/".join(parts[parts.index("crates") + 1 :]) if "crates" in parts else path


def cell(summary: dict, kind: str) -> str:
    count = summary[kind]["count"]
    covered = summary[kind]["covered"]
    if count == 0:
        return "–"
    return f"{covered / count * 100:.2f}% ({count - covered} missed of {count})"


def add(into: dict, summary: dict) -> None:
    for kind in ("lines", "branches"):
        into.setdefault(kind, {"count": 0, "covered": 0})
        into[kind]["count"] += summary[kind]["count"]
        into[kind]["covered"] += summary[kind]["covered"]


def percent(summary: dict, kind: str) -> float:
    count = summary[kind]["count"]
    return 100.0 if count == 0 else summary[kind]["covered"] / count * 100


def table(export: dict) -> tuple[list[str], dict]:
    data = export["data"][0]
    crates: dict[str, dict] = {}
    rows = []
    for entry in data["files"]:
        add(crates.setdefault(crate_of(entry["filename"]), {}), entry["summary"])
        rows.append((entry["summary"]["lines"]["count"] - entry["summary"]["lines"]["covered"], short(entry["filename"]), entry["summary"]))
    totals = data["totals"]
    lines = [
        f"Coverage: {cell(totals, 'lines')} lines, {cell(totals, 'branches')} branches; "
        f"floors {LINES_FLOOR} / {BRANCHES_FLOOR}",
        "",
        "| Crate or file | Lines | Branches |",
        "|---|---|---|",
    ]
    for name in sorted(crates):
        lines.append(f"| **{name}** | {cell(crates[name], 'lines')} | {cell(crates[name], 'branches')} |")
    for missed, name, summary in sorted(rows, key=lambda row: (-row[0], row[1])):
        if missed or summary["branches"]["count"] > summary["branches"]["covered"]:
            lines.append(f"| `{name}` | {cell(summary, 'lines')} | {cell(summary, 'branches')} |")
    return lines, totals


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
    parser.add_argument("summary", type=Path, help="the --json --summary-only export of cargo-llvm-cov")
    parser.add_argument("--repo", type=Path, default=Path("."), help="the repo root (default: the current folder)")
    args = parser.parse_args()

    export = json.loads(args.summary.read_text(encoding="utf-8"))
    lines, totals = table(export)
    print("\n".join(lines))
    failures: list[str] = []
    if percent(totals, "lines") < LINES_FLOOR:
        failures.append(f"lines {percent(totals, 'lines'):.2f}% are below the floor of {LINES_FLOOR}%")
    if percent(totals, "branches") < BRANCHES_FLOOR:
        failures.append(f"branches {percent(totals, 'branches'):.2f}% are below the floor of {BRANCHES_FLOOR}%")

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
