"""Mirrors the task files into GitHub issues and milestones, one way (the project-pal task standard).

The task files are the source of truth (`tasks/` or `.claude/tasks/`, its README.md). Every task
(backlog/ wip/ done/ dropped/) and every decision (decisions/D-NNN-*.md) is one issue: the id in the
title, the Context prose in the body, the milestone from the frontmatter, labels for stage / status /
size, open or closed by the folder. Every `## ` heading of roadmap.md is a milestone (its state is
never touched here: a release closes a milestone). Idempotent: an issue is found by the marker in its
body and only the fields that differ are patched; an id padded to another width since its issue was
made (`/tasks renumber`) still finds that issue, which is edited in place, marker included; a second
issue of one file (a duplicate, whatever run made it) is closed as not planned with a comment naming the
one kept, the lowest-numbered; an issue whose file is gone is reported, never closed or deleted; a file
without a numbered name (a legacy file the standard tolerates) is reported and not mirrored. Run by CI on a push that touches the task files (.github/workflows/sync-issues.yml),
or by hand after a push:

    python -I scripts/sync-issues.py --dry-run     # print what would change
    python -I scripts/sync-issues.py               # apply
    python -I scripts/sync-issues.py --check       # exit 1 when anything would change

Needs `gh` (logged in, or GH_TOKEN in CI); nothing beyond the standard library. This file comes from
the project-pal tasks skill (`/tasks mirror`); TEMPLATE_VERSION tells a later run whether it is older.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

TEMPLATE_VERSION = "3"

ROOT = Path(__file__).resolve().parent.parent
TASK_LOCATIONS = ("tasks", ".claude/tasks")
STAGES = ("backlog", "wip", "done", "dropped")
TASK_FILE = re.compile(r"^(?!\d{4}-\d{2}-\d{2}-)(\d{2,4})([a-z]?)-[a-z0-9]+(?:-[a-z0-9]+)*\.md$")
DECISION_FILE = re.compile(r"^D-(\d{2,4})-[a-z0-9]+(?:-[a-z0-9]+)*\.md$")
MARKER = re.compile(r"<!-- sync-issues: (task|decision) (\S+) -->")
MIRROR_LABEL = "mirrored"
SCRIPT = "scripts/sync-issues.py"


def locate_tasks(root: Path) -> Path:
    """`tasks/` first, then `.claude/tasks/` — the standard's two locations, in its order."""
    for location in TASK_LOCATIONS:
        if (root / location).is_dir():
            return root / location
    raise SystemExit(f"{root}: no tasks/ or .claude/tasks/ folder")


TASKS = ROOT / "tasks"  # set by main() through locate_tasks(); importers of this module get the default
TASKS_REL = "tasks"


def labels_table() -> dict[str, tuple[str, str]]:
    t = TASKS_REL
    return {
        MIRROR_LABEL: ("ededed", f"Mirrored from {t}/ by {SCRIPT}; edit the file, not the issue"),
        "stage:backlog": ("c2e0c6", f"Open, not started ({t}/backlog/)"),
        "stage:wip": ("fbca04", f"Work in flight ({t}/wip/)"),
        "status:idea": ("bfd4f2", "Backlog status: idea"),
        "status:ready": ("bfd4f2", "Backlog status: ready"),
        "status:blocked": ("bfd4f2", "Backlog status: blocked on a decision"),
        "status:waiting": ("bfd4f2", "Backlog status: waiting on a date or condition"),
        "size:S": ("d4c5f9", "Up to half a day"),
        "size:M": ("d4c5f9", "One or two days"),
        "size:L": ("d4c5f9", "More than two days"),
        "dropped": ("cfd3d7", f"Closed without work ({t}/dropped/)"),
        "decision": ("0e8a16", f"A decision record ({t}/decisions/)"),
    }


# --- the flat frontmatter subset of the task standard ---------------------------------------


def _cut_comment(raw: str) -> str:
    value = raw.strip()
    if value[:1] in ('"', "'"):
        quote = value[0]
        i = 1
        while i < len(value):
            if value[i] == "\\" and quote == '"':
                i += 2
                continue
            if value[i] == quote:
                if quote == "'" and value[i + 1 : i + 2] == "'":
                    i += 2
                    continue
                break
            i += 1
        return value[: i + 1]
    match = re.search(r"\s#", value)
    return (value if match is None else value[: match.start()]).strip()


def _unquote(raw: str) -> str:
    value = raw.strip()
    if len(value) >= 2 and value[0] == value[-1] == '"':
        return re.sub(r'\\(["\\])', r"\1", value[1:-1])
    if len(value) >= 2 and value[0] == value[-1] == "'":
        return value[1:-1].replace("''", "'")
    return value


def _split_inline(inner: str) -> list[str]:
    out: list[str] = []
    current = ""
    quote: str | None = None
    for ch in inner:
        if quote is not None:
            current += ch
            if ch == quote:
                quote = None
        elif ch in ('"', "'"):
            quote = ch
            current += ch
        elif ch == ",":
            out.append(current)
            current = ""
        else:
            current += ch
    out.append(current)
    return [v for v in (_unquote(_cut_comment(s)) for s in out) if v != ""]


def parse_frontmatter(text: str, where: str) -> tuple[dict[str, str | list[str]], str]:
    lines = text.lstrip("﻿").splitlines()
    if not lines or lines[0].strip() != "---":
        raise SystemExit(f"{where}: no frontmatter")
    end = next((i for i in range(1, len(lines)) if lines[i].strip() in ("---", "...")), None)
    if end is None:
        raise SystemExit(f"{where}: frontmatter fence never closes")
    data: dict[str, str | list[str]] = {}
    open_key: str | None = None
    for i in range(1, end):
        line = lines[i]
        if line.strip() == "" or line.strip().startswith("#"):
            continue
        if line[0] in " \t":
            item = re.match(r"^[ \t]+-[ \t]+(.*)$", line)
            if open_key is None or item is None:
                raise SystemExit(f"{where}: line {i + 1}: cannot read {line.strip()!r}")
            current = data[open_key]
            items = current if isinstance(current, list) else []
            value = _unquote(_cut_comment(item.group(1)))
            if value:
                items.append(value)
            data[open_key] = items
            continue
        kv = re.match(r"^([A-Za-z0-9_][A-Za-z0-9_-]*):(?:[ \t]+(.*))?$", line)
        if kv is None:
            raise SystemExit(f"{where}: line {i + 1}: cannot read {line.strip()!r}")
        key, raw = kv.group(1), _cut_comment(kv.group(2) or "")
        if raw == "":
            data[key] = ""
            open_key = key
            continue
        open_key = None
        if raw.startswith("[") and raw.endswith("]"):
            data[key] = _split_inline(raw[1:-1])
        elif raw[:1] in "|>{":
            raise SystemExit(f"{where}: `{key}`: block scalars and inline maps are not supported")
        else:
            data[key] = _unquote(raw)
    return data, "\n".join(lines[end + 1 :])


def scalar(data: dict[str, str | list[str]], key: str) -> str | None:
    value = data.get(key)
    return value if isinstance(value, str) and value != "" else None


# --- the items ------------------------------------------------------------------------------


@dataclass
class Item:
    kind: str  # "task" | "decision"
    key: str  # "021" | "D-008"
    sort: tuple[int, str]
    rel_path: str  # "done/021-slug.md"
    title: str
    body: str
    state: str  # "open" | "closed"
    state_reason: str | None
    milestone: str | None
    labels: list[str] = field(default_factory=list)


def _prose(body: str) -> list[str]:
    """Body lines outside code fences."""
    out: list[str] = []
    fence = False
    for line in body.splitlines():
        if re.match(r"^\s*(```|~~~)", line):
            fence = not fence
            continue
        if not fence:
            out.append(line)
    return out


def _heading(body: str, where: str) -> str:
    for line in _prose(body):
        match = re.match(r"^#\s+(\S.*)$", line)
        if match:
            return match.group(1).strip()
    raise SystemExit(f"{where}: no H1")


def _title(key: str, heading: str) -> str:
    bare = re.sub(r"^(\d{2,4}[a-z]?|D-\d{2,4})\s*[—–-]\s*", "", heading)
    return f"{key} — {bare}"


def _lead(body: str) -> str:
    """The prose that describes the item: the `## Context` section when there is one, else the lines
    between the H1 and the first `## ` heading (a backlog item, or a done file still in backlog shape)."""
    lines = body.splitlines()
    start = next((i + 1 for i, l in enumerate(lines) if re.match(r"^##\s+Context\s*$", l)), None)
    if start is None:
        start = next((i + 1 for i, l in enumerate(lines) if re.match(r"^#\s+", l)), 0)
    end = next((i for i in range(start, len(lines)) if re.match(r"^##\s+", lines[i])), len(lines))
    text = re.sub(r"<!--.*?-->", "", "\n".join(lines[start:end]), flags=re.S)
    return _trim(text.splitlines())


def _whole(body: str) -> str:
    """Everything after the H1 (a decision record: Context · Decision · Not chosen)."""
    lines = body.splitlines()
    start = next((i + 1 for i, l in enumerate(lines) if re.match(r"^#\s+", l)), 0)
    text = re.sub(r"<!--.*?-->", "", "\n".join(lines[start:]), flags=re.S)
    return _trim(text.splitlines())


def _trim(lines: list[str]) -> str:
    while lines and lines[0].strip() == "":
        lines.pop(0)
    while lines and lines[-1].strip() == "":
        lines.pop()
    return "\n".join(lines)


def _meta_line(pairs: list[tuple[str, str | None]]) -> str:
    return " · ".join(f"**{name}:** {value}" for name, value in pairs if value)


def _body(blob: str, item_kind: str, key: str, rel_path: str, meta: str, prose: str) -> str:
    """`blob` is the file link's base: https://github.com/<repo>/blob/<branch>/<tasks dir>."""
    head = (
        f"<!-- sync-issues: {item_kind} {key} -->\n"
        f"> Mirrored from [`{TASKS_REL}/{rel_path}`]({blob}/{rel_path}) by `{SCRIPT}` — edit the file, not this issue; "
        f"comments are welcome here.\n\n{meta}"
    )
    return f"{head}\n\n{prose}" if prose else head


def _skip(rel: str, what: str) -> None:
    print(f"skip       {TASKS_REL}/{rel}: not a numbered {what} file name — not mirrored")


def load_tasks(blob: str) -> list[Item]:
    items: list[Item] = []
    for stage in STAGES:
        folder = TASKS / stage
        if not folder.is_dir():
            continue
        for path in sorted(folder.iterdir()):
            if path.suffix != ".md":
                continue
            rel = f"{stage}/{path.name}"
            match = TASK_FILE.match(path.name)
            if match is None:
                _skip(rel, "task")
                continue
            data, body = parse_frontmatter(path.read_text(encoding="utf-8"), rel)
            key = match.group(1) + match.group(2)
            size = scalar(data, "size")
            labels = [MIRROR_LABEL] + ([f"size:{size}"] if size else [])
            pairs: list[tuple[str, str | None]] = [("Stage", stage)]
            if stage == "backlog":
                status = scalar(data, "status")
                labels += ["stage:backlog"] + ([f"status:{status}"] if status else [])
                pairs += [("Status", status), ("Needs", scalar(data, "needs")), ("Waits for", scalar(data, "waits-for"))]
            if stage == "wip":
                labels.append("stage:wip")
            if stage == "dropped":
                labels.append("dropped")
            pairs += [
                ("Size", size),
                ("Milestone", scalar(data, "milestone")),
                ("Created", scalar(data, "created")),
                ("Started", scalar(data, "started")),
                ("Closed", scalar(data, "closed")),
                ("Origin", scalar(data, "origin")),
                ("Reason", scalar(data, "reason")),
                ("Reopen when", scalar(data, "reopen-when")),
            ]
            closed = stage in ("done", "dropped")
            items.append(
                Item(
                    kind="task",
                    key=key,
                    sort=(0, key.zfill(6)),
                    rel_path=rel,
                    title=_title(key, _heading(body, rel)),
                    body=_body(blob, "task", key, rel, _meta_line(pairs), _lead(body)),
                    state="closed" if closed else "open",
                    state_reason=("not_planned" if stage == "dropped" else "completed") if closed else None,
                    milestone=scalar(data, "milestone"),
                    labels=sorted(labels),
                )
            )
    return items


def load_decisions(blob: str) -> list[Item]:
    items: list[Item] = []
    folder = TASKS / "decisions"
    if not folder.is_dir():
        return items
    for path in sorted(folder.iterdir()):
        if path.suffix != ".md":
            continue
        rel = f"decisions/{path.name}"
        match = DECISION_FILE.match(path.name)
        if match is None:
            _skip(rel, "decision")
            continue
        data, body = parse_frontmatter(path.read_text(encoding="utf-8"), rel)
        key = f"D-{match.group(1)}"
        status = scalar(data, "status") or "proposed"
        sources = data.get("sources")
        sources_text = ", ".join(sources) if isinstance(sources, list) else (sources or None)
        pairs: list[tuple[str, str | None]] = [
            ("Status", status),
            ("Date", scalar(data, "date")),
            ("Cost", scalar(data, "cost")),
            ("Supersedes", scalar(data, "supersedes")),
            ("Superseded by", scalar(data, "superseded-by")),
            ("Sources", sources_text),
        ]
        closed = status != "proposed"
        items.append(
            Item(
                kind="decision",
                key=key,
                sort=(1, match.group(1).zfill(6)),
                rel_path=rel,
                title=_title(key, _heading(body, rel)),
                body=_body(blob, "decision", key, rel, _meta_line(pairs), _whole(body)),
                state="closed" if closed else "open",
                state_reason=("not_planned" if status == "superseded" else "completed") if closed else None,
                milestone=None,
                labels=sorted([MIRROR_LABEL, "decision"]),
            )
        )
    return items


def load_roadmap() -> dict[str, str]:
    """`## heading` → its paragraph, in roadmap order."""
    path = TASKS / "roadmap.md"
    if not path.is_file():
        return {}
    out: dict[str, str] = {}
    current: str | None = None
    for line in _prose(path.read_text(encoding="utf-8")):
        match = re.match(r"^##\s+(\S.*)$", line)
        if match:
            current = match.group(1).strip()
            out[current] = ""
        elif current is not None and not line.startswith("#"):
            out[current] = (out[current] + " " + line.strip()).strip()
    return out


# --- GitHub ---------------------------------------------------------------------------------


def gh(*args: str, payload: dict | None = None) -> object:
    cmd = ["gh", *args]
    stdin = None
    if payload is not None:
        cmd += ["--input", "-"]
        stdin = json.dumps(payload)
    # cwd=ROOT: without --repo, gh resolves the remote of the repo being mirrored, not of the caller's cwd.
    result = subprocess.run(cmd, input=stdin, capture_output=True, text=True, encoding="utf-8", cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(f"gh {' '.join(args)} failed ({result.returncode}):\n{result.stderr.strip()}\n{result.stdout.strip()}")
    return json.loads(result.stdout) if result.stdout.strip() else None


def paged(repo: str, path: str, **params: str) -> list[dict]:
    out: list[dict] = []
    page = 1
    while True:
        query = "&".join([f"{k}={v}" for k, v in params.items()] + ["per_page=100", f"page={page}"])
        chunk = gh("api", f"repos/{repo}/{path}?{query}")
        assert isinstance(chunk, list)
        out += chunk
        if len(chunk) < 100:
            return out
        page += 1


def repo_view(repo: str | None) -> dict:
    """`nameWithOwner` and the default branch, of `repo` or of the working copy's remote."""
    view = gh("repo", "view", *([repo] if repo else []), "--json", "nameWithOwner,defaultBranchRef")
    assert isinstance(view, dict)
    return view


def _norm(text: str | None) -> str:
    return (text or "").replace("\r\n", "\n").rstrip()


def same_id(key: str) -> str:
    """The key without the zero padding of its number: "D-8", "D-0008" and "D-008" are one decision."""
    return re.sub(r"\d+", lambda m: str(int(m.group())), key, count=1)


def match_issues(items: list[Item], existing_by_key: dict[tuple[str, str], list[dict]]) -> dict[tuple[str, str], tuple[dict, str] | None]:
    """(kind, key) of every item → (its issue, the key in that issue's marker), or None for a new one.
    The marker's exact key wins; else an issue whose marker holds the same number at another width
    (a repo that mirrored before `/tasks renumber` padded its ids keeps its issues, edited in place).
    Each key's issues come sorted by number and the lowest one is taken, so a run always keeps the same
    issue; what stays in `existing_by_key` is a duplicate (a key an item has) or an orphan (no file)."""
    out: dict[tuple[str, str], tuple[dict, str] | None] = {}

    def take(key: tuple[str, str]) -> dict | None:
        issues = existing_by_key.get(key)
        if not issues:
            return None
        issue = issues.pop(0)
        if not issues:
            del existing_by_key[key]
        return issue

    for item in items:
        issue = take((item.kind, item.key))
        out[(item.kind, item.key)] = None if issue is None else (issue, item.key)
    for item in items:
        if out[(item.kind, item.key)] is not None:
            continue
        old = next((k for k in existing_by_key if k[0] == item.kind and same_id(k[1]) == same_id(item.key)), None)
        if old is not None:
            issue = take(old)
            assert issue is not None
            out[(item.kind, item.key)] = (issue, old[1])
    return out


# --- the sync -------------------------------------------------------------------------------


def sync(repo: str, branch: str, apply: bool) -> int:
    """Returns the number of changes planned (dry run) or made (apply)."""
    blob = f"https://github.com/{repo}/blob/{branch}/{TASKS_REL}"
    items = sorted(load_tasks(blob) + load_decisions(blob), key=lambda i: i.sort)
    roadmap = load_roadmap()
    changes = 0
    log = print

    if apply:
        for name, (color, description) in labels_table().items():
            gh("label", "create", name, "--repo", repo, "--color", color, "--description", description, "--force")

    milestones: dict[str, dict] = {m["title"]: m for m in paged(repo, "milestones", state="all")}
    for title, description in roadmap.items():
        existing = milestones.get(title)
        if existing is None:
            log(f"milestone  create   {title}")
            changes += 1
            if apply:
                created = gh("api", f"repos/{repo}/milestones", payload={"title": title, "description": description})
                assert isinstance(created, dict)
                milestones[title] = created
            else:
                # A dry run still plans the issues under the milestone it would create.
                milestones[title] = {"title": title, "number": None}
        elif _norm(existing.get("description")) != _norm(description):
            log(f"milestone  update   {title} (description)")
            changes += 1
            if apply:
                gh("api", "-X", "PATCH", f"repos/{repo}/milestones/{existing['number']}", payload={"description": description})
    for item in items:
        if item.milestone is not None and item.milestone not in milestones:
            raise SystemExit(f"{TASKS_REL}/{item.rel_path}: milestone {item.milestone!r} is not a `## ` heading of {TASKS_REL}/roadmap.md")

    existing_by_key: dict[tuple[str, str], list[dict]] = {}
    for issue in paged(repo, "issues", state="all", labels=MIRROR_LABEL):
        if "pull_request" in issue:
            continue
        match = MARKER.search(issue.get("body") or "")
        if match:
            existing_by_key.setdefault((match.group(1), match.group(2)), []).append(issue)
    for issues in existing_by_key.values():
        issues.sort(key=lambda issue: issue["number"])

    matched = match_issues(items, existing_by_key)
    kept_by_id = {(kind, same_id(key)): found[0]["number"] for (kind, key), found in matched.items() if found is not None}
    created = updated = unchanged = duplicates = 0
    for item in items:
        found = matched[(item.kind, item.key)]
        issue = None if found is None else found[0]
        milestone_number = milestones[item.milestone]["number"] if item.milestone else None
        if issue is None:
            log(f"issue      create   {item.title}  [{item.state}, {', '.join(item.labels)}]")
            created += 1
            if apply:
                payload: dict = {"title": item.title, "body": item.body, "labels": item.labels}
                if milestone_number is not None:
                    payload["milestone"] = milestone_number
                made = gh("api", f"repos/{repo}/issues", payload=payload)
                assert isinstance(made, dict)
                if item.state == "closed":
                    gh("api", "-X", "PATCH", f"repos/{repo}/issues/{made['number']}", payload={"state": "closed", "state_reason": item.state_reason})
            continue
        patch: dict = {}
        if issue.get("title") != item.title:
            patch["title"] = item.title
        if _norm(issue.get("body")) != _norm(item.body):
            patch["body"] = item.body
        have_milestone = (issue.get("milestone") or {}).get("number")
        if have_milestone != milestone_number:
            patch["milestone"] = milestone_number
        have_labels = sorted(label["name"] for label in issue.get("labels", []))
        if have_labels != item.labels:
            patch["labels"] = item.labels
        if issue.get("state") != item.state or (item.state == "closed" and issue.get("state_reason") != item.state_reason):
            patch["state"] = item.state
            if item.state == "closed":
                patch["state_reason"] = item.state_reason
        if not patch:
            unchanged += 1
            continue
        log(f"issue      update   #{issue['number']} {item.title}  ({', '.join(sorted(patch))})")
        updated += 1
        if apply:
            gh("api", "-X", "PATCH", f"repos/{repo}/issues/{issue['number']}", payload=patch)

    for (kind, key), issues in sorted(existing_by_key.items()):
        kept = kept_by_id.get((kind, same_id(key)))
        for issue in issues:
            if kept is None:
                log(f"orphan     #{issue['number']} {kind} {key}: no file under {TASKS_REL}/ — left as is")
            elif issue.get("state") == "closed":
                log(f"duplicate  #{issue['number']} {kind} {key} of #{kept}: already closed")
            else:
                log(f"duplicate  #{issue['number']} {kind} {key} of #{kept}: closed as not planned")
                duplicates += 1
                if apply:
                    note = f"Duplicate of #{kept}, created by a second mirror run; the {kind}'s issue is #{kept}."
                    gh("api", f"repos/{repo}/issues/{issue['number']}/comments", payload={"body": note})
                    gh("api", "-X", "PATCH", f"repos/{repo}/issues/{issue['number']}", payload={"state": "closed", "state_reason": "not_planned"})

    verb = "" if apply else " (dry run)"
    log(f"{'applied' if apply else 'planned'}{verb}: created {created}, updated {updated}, unchanged {unchanged}, "
        f"duplicates closed {duplicates}; "
        f"{len(items)} items ({sum(i.kind == 'task' for i in items)} tasks, {sum(i.kind == 'decision' for i in items)} decisions)")
    return changes + created + updated + duplicates


def main() -> int:
    global ROOT, TASKS, TASKS_REL
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # titles carry em dashes; Windows consoles may not
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", help="owner/name (default: the repo `gh repo view` resolves from the working copy)")
    parser.add_argument("--branch", help="the branch the file links point at (default: the repo's default branch)")
    parser.add_argument("--root", type=Path, help="the repo whose task files are read (default: the one this script sits in)")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", action="store_true", help="print the planned changes, write nothing")
    mode.add_argument("--check", action="store_true", help="like --dry-run, exit 1 when anything would change")
    mode.add_argument("--version", action="store_true", help="print TEMPLATE_VERSION and exit")
    args = parser.parse_args()
    if args.version:
        print(TEMPLATE_VERSION)
        return 0
    if args.root is not None:
        ROOT = args.root.resolve()
    TASKS = locate_tasks(ROOT)
    TASKS_REL = TASKS.relative_to(ROOT).as_posix()
    view = repo_view(args.repo)
    branch = args.branch or (view.get("defaultBranchRef") or {}).get("name") or "main"
    changed = sync(view["nameWithOwner"], branch, apply=not (args.dry_run or args.check))
    return 1 if args.check and changed else 0


if __name__ == "__main__":
    sys.exit(main())
