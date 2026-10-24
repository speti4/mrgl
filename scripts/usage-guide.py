"""Renders GUIDE.html, the usage guide of the release zip, from the user sections of README.md.

The README is the one source of the words: the page is generated when a release is packed and never committed
or edited by hand. scripts/release.ps1 writes it next to the exes; CI runs `--check`, which renders the page
from the current README and writes nothing, so a README edit the generator cannot read fails the pull request
that makes it, not the release.

HTML comments in the README, which GitHub does not show, say which parts the page takes:

    <!-- guide: start -->            the README from here on is on the page,
    <!-- guide: end -->              up to here; the regions follow each other on the page
    <!-- guide: section <title> -->  a heading on the page only (inside a region), for README text that sits
                                     under a heading the page leaves out or that the page splits up

The page is one self-contained file: inline CSS, no script, no image, no external asset, so it opens offline.
Its header names the version (the [workspace.package] version of Cargo.toml, as scripts/release.ps1 reads it)
with the text of the regions before their first heading, then a table of contents of the headings, then the
rest. Two rewrites fit the README's words to a page that sits in the unpacked zip: `target\\release\\`, the
folder of a build from source, becomes `.\\`, the folder of the exes; and a link to a file of the repo points
at that file in the release's tag on GitHub (a link to a heading the page has stays on the page).

The Markdown is the subset the README uses, read with the standard library alone (D-002): `##` and `###`
headings, paragraphs, `- ` lists with indented continuation lines, fenced code, code spans, links, `**bold**`
and backslash escapes. Anything else in a region (a table, a quote, a numbered or nested list, emphasis, an
image, raw HTML) stops the run with its line number, instead of a page that renders it wrong. The output is
deterministic: the same README and version give the same bytes.

    python -I scripts/usage-guide.py --out dist/GUIDE.html   # write the page
    python -I scripts/usage-guide.py --check                 # render it, write nothing; exit 1 on a problem
"""

from __future__ import annotations

import argparse
import html
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = ROOT / "README.md"
CARGO_TOML = ROOT / "Cargo.toml"

MARKER = re.compile(r"^<!--\s*guide:\s*(.*?)\s*-->$")
FENCE = re.compile(r"^```")
HEADING = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$")
ITEM = re.compile(r"^- (.*)$")
# Block starts the page does not render; each one names what it is in the error.
UNSUPPORTED_BLOCKS = [
    (re.compile(r"^\s*\|"), "a table"),
    (re.compile(r"^\s*>"), "a block quote"),
    (re.compile(r"^\s*\d+[.)]\s"), "a numbered list"),
    (re.compile(r"^\s*[*+]\s"), "a list item with `*` or `+` (the page reads `- ` items)"),
    (re.compile(r"^\s+[-*+]\s"), "a nested list"),
    (re.compile(r"^ {4,}\S"), "an indented code block (the page reads fenced code)"),
    (re.compile(r"^\s*(?:-{3,}|\*{3,}|_{3,}|={3,})\s*$"), "a thematic break or a setext heading"),
    (re.compile(r"^\s*<"), "raw HTML"),
]
CODE_SPAN = re.compile(r"(`+)(.+?)(?<!`)\1(?!`)", re.S)
LINK = re.compile(r"\[((?:[^\[\]\\`]|\\.|`[^`]*`)*)\]\(([^()\s]+)\)")
BOLD = re.compile(r"\*\*(?=\S)(.+?)(?<=\S)\*\*", re.S)
ESCAPE = re.compile(r"\\([!-/:-@\[-`{-~])")
# What a plain run of text must not hold once the code spans, links and bold are taken out.
UNSUPPORTED_INLINE = [
    (re.compile(r"!\["), "an image"),
    (re.compile(r"\*"), "emphasis with `*` (the page reads `**bold**` only)"),
    (re.compile(r"(?<![\w\\])_(?=\S)[^_]*(?<=\S)_(?!\w)"), "emphasis with `_`"),
    (re.compile(r"<[A-Za-z/!?]"), "raw HTML or an autolink"),
    (re.compile(r"\]\(|\]\["), "a link the page cannot read"),
]
RELEASE_FOLDER = "target\\release\\"
EXE_FOLDER = ".\\"
BANNER = "Mrglglgl."  # the banner line of the voice table (crates/mrgl-core/src/voice.rs), the greeting

STYLE = """\
  body { font: 15px/1.55 "Segoe UI", system-ui, sans-serif; max-width: 60rem; margin: 2rem auto; padding: 0 1rem; color: #1f2328; background: #fff; }
  h1 { font-size: 1.7rem; margin: 0.2rem 0 1rem; }
  h2 { font-size: 1.3rem; margin-top: 2.5rem; padding-bottom: 0.2rem; border-bottom: 1px solid #d0d7de; }
  h3 { font-size: 1.1rem; margin-top: 1.8rem; }
  .murloc { margin: 0; color: #59636e; font-style: italic; }
  nav { margin: 1.5rem 0 2rem; padding: 0.8rem 1.2rem; background: #f6f8fa; border-radius: 6px; }
  nav h2 { margin: 0 0 0.4rem; font-size: 1rem; border: 0; }
  nav ol { margin: 0; padding-left: 1.4rem; columns: 2; }
  code { font: 13px/1.45 Consolas, "Cascadia Mono", monospace; background: #eff1f3; padding: 0.1rem 0.3rem; border-radius: 4px; }
  pre { background: #f6f8fa; padding: 0.9rem 1rem; overflow-x: auto; border-radius: 6px; }
  pre code { background: none; padding: 0; white-space: pre; }
  a { color: #0969da; }
  footer { margin-top: 3rem; padding-top: 0.8rem; border-top: 1px solid #d0d7de; color: #59636e; font-size: 13px; }
  @media (max-width: 40rem) { nav ol { columns: 1; } }
  @media (prefers-color-scheme: dark) {
    body { color: #e6edf3; background: #0d1117; }
    h2, footer { border-color: #30363d; }
    .murloc, footer { color: #9198a1; }
    nav, pre { background: #161b22; }
    code { background: #262c36; }
    a { color: #58a6ff; }
  }
"""


class GuideError(Exception):
    """A README the page cannot be rendered from; the message names the line."""


@dataclass
class Line:
    number: int  # 1-based, in README.md
    text: str


@dataclass
class Heading:
    level: int
    text: str  # the Markdown of the heading
    slug: str


@dataclass
class Page:
    lead: list[str] = field(default_factory=list)  # the HTML blocks before the first heading
    body: list[str] = field(default_factory=list)  # the HTML blocks from the first heading on
    headings: list[Heading] = field(default_factory=list)


# --- the inputs -----------------------------------------------------------------------------


def workspace_version(cargo_toml: str) -> str:
    """The [workspace.package] version, read as scripts/release.ps1 reads it."""
    match = re.search(r'(?ms)^\[workspace\.package\][^\[]*?^version = "([^"]+)"', cargo_toml)
    if match is None:
        raise GuideError("Cargo.toml: no [workspace.package] version")
    return match.group(1)


def repo_url(cargo_toml: str) -> str:
    match = re.search(r'(?m)^repository = "([^"]+)"', cargo_toml)
    if match is None:
        raise GuideError("Cargo.toml: no `repository` in [workspace.package]")
    return match.group(1).rstrip("/")


def regions(readme: str) -> list[Line]:
    """The README lines between the markers, in order, a `section` marker kept as a line of its own; a marker
    inside fenced code is text."""
    picked: list[Line] = []
    inside = False
    fenced = False
    found = False
    for number, text in enumerate(readme.replace("\r\n", "\n").split("\n"), start=1):
        if FENCE.match(text):
            fenced = not fenced
        marker = None if fenced else MARKER.match(text.strip())
        if marker is None:
            if "guide:" in text and text.lstrip().startswith("<!--") and not fenced:
                raise GuideError(f"README.md:{number}: a guide marker the page cannot read: {text.strip()}")
            if inside:
                picked.append(Line(number, text.replace(RELEASE_FOLDER, EXE_FOLDER)))
            continue
        word, _, rest = marker.group(1).partition(" ")
        if word == "start" and not rest:
            if inside:
                raise GuideError(f"README.md:{number}: `guide: start` inside a region")
            inside = found = True
        elif word == "end" and not rest:
            if not inside:
                raise GuideError(f"README.md:{number}: `guide: end` without a start")
            inside = False
            picked.append(Line(number, ""))  # a region ends its last block
        elif word == "section" and rest.strip():
            if not inside:
                raise GuideError(f"README.md:{number}: `guide: section` outside a region")
            picked.append(Line(number, text.strip()))
        else:
            raise GuideError(f"README.md:{number}: an unknown guide marker: {text.strip()}")
    if inside:
        raise GuideError("README.md: a `guide: start` without its end")
    if fenced:
        raise GuideError("README.md: a code fence without its end")
    if not found:
        raise GuideError("README.md: no `<!-- guide: start -->` marker, so the page would be empty")
    return picked


# --- the inline Markdown --------------------------------------------------------------------


@dataclass
class Links:
    """Where a link of the README goes on the page."""

    base: str  # the release's tag on GitHub, "<repo>/blob/v<version>"
    tree: str  # the same for a folder, "<repo>/tree/v<version>"
    readme: str  # the README there, for an anchor the page does not have
    slugs: set[str]

    def href(self, target: str, where: str) -> str:
        if re.match(r"^[a-z][a-z0-9+.-]*:", target):
            return target
        if target.startswith("#"):
            return target if target[1:] in self.slugs else self.readme + target
        path, _, anchor = target.partition("#")
        local = (ROOT / path).resolve()
        if not local.is_relative_to(ROOT) or not local.exists():
            raise GuideError(f"{where}: the link `{target}` names no file of the repo")
        base = self.tree if local.is_dir() else self.base
        return f"{base}/{path.rstrip('/')}" + (f"#{anchor}" if anchor else "")


def plain(text: str, where: str) -> str:
    unescaped = ESCAPE.sub("", text)
    for pattern, what in UNSUPPORTED_INLINE:
        if pattern.search(unescaped):
            raise GuideError(f"{where}: {what} in {text.strip()!r}")
    parts: list[str] = []
    last = 0
    for match in ESCAPE.finditer(text):
        parts.append(html.escape(text[last : match.start()], quote=False))
        parts.append(html.escape(match.group(1), quote=False))
        last = match.end()
    parts.append(html.escape(text[last:], quote=False))
    return "".join(parts)


def inline(text: str, links: Links, where: str) -> str:
    """Code spans, links and bold, the earliest first; the text between them is plain."""
    out: list[str] = []
    rest = text
    while rest:
        found = [m for m in (CODE_SPAN.search(rest), LINK.search(rest), BOLD.search(rest)) if m is not None]
        if not found:
            out.append(plain(rest, where))
            break
        match = min(found, key=lambda m: m.start())
        out.append(plain(rest[: match.start()], where))
        if match.re is CODE_SPAN:
            code = match.group(2).replace("\n", " ")
            if code.startswith(" ") and code.endswith(" ") and code.strip():
                code = code[1:-1]
            out.append(f"<code>{html.escape(code, quote=False)}</code>")
        elif match.re is LINK:
            if rest[: match.start()].endswith("!"):
                raise GuideError(f"{where}: an image in {text.strip()!r}")
            href = html.escape(links.href(match.group(2), where))
            out.append(f'<a href="{href}">{inline(match.group(1), links, where)}</a>')
        else:
            out.append(f"<strong>{inline(match.group(1), links, where)}</strong>")
        rest = rest[match.end() :]
    return "".join(out)


def slug(text: str, taken: set[str]) -> str:
    """GitHub's anchor of a heading: lower case, punctuation dropped, spaces as hyphens, a repeat numbered."""
    base = re.sub(r"[^\w\- ]", "", text.strip().lower()).replace(" ", "-")
    candidate, n = base, 0
    while candidate in taken:
        n += 1
        candidate = f"{base}-{n}"
    taken.add(candidate)
    return candidate


# --- the blocks -----------------------------------------------------------------------------


def headings_of(lines: list[Line]) -> list[Heading]:
    """Every heading of the regions, with its anchor; the anchors are needed before the text is rendered,
    since a link may point forward."""
    taken = {"contents"}  # the table of contents' own anchor
    found: list[Heading] = []
    fenced = False
    for line in lines:
        if FENCE.match(line.text):
            fenced = not fenced
            continue
        if fenced:
            continue
        marker = MARKER.match(line.text)
        if marker:
            title = marker.group(1).partition(" ")[2].strip()
            found.append(Heading(2, title, slug(title, taken)))
            continue
        heading = HEADING.match(line.text)
        if heading:
            level = len(heading.group(1))
            if level not in (2, 3):
                raise GuideError(f"README.md:{line.number}: a level-{level} heading (the page reads ## and ###)")
            found.append(Heading(level, heading.group(2), slug(heading.group(2), taken)))
    return found


def starts_block(text: str) -> bool:
    return bool(FENCE.match(text) or HEADING.match(text) or ITEM.match(text) or MARKER.match(text))


def check_unsupported(line: Line) -> None:
    for pattern, what in UNSUPPORTED_BLOCKS:
        if pattern.match(line.text):
            raise GuideError(f"README.md:{line.number}: {what}: {line.text.strip()}")


def render(lines: list[Line], links: Links, headings: list[Heading]) -> Page:
    page = Page(headings=headings)
    queue = iter(headings)
    out = page.lead
    i = 0
    while i < len(lines):
        line = lines[i]
        where = f"README.md:{line.number}"
        if not line.text.strip():
            i += 1
            continue
        if FENCE.match(line.text):
            code: list[str] = []
            i += 1
            while not FENCE.match(lines[i].text):
                code.append(lines[i].text)
                i += 1
            i += 1
            body = html.escape("\n".join(code), quote=False)
            out.append(f"<pre><code>{body}</code></pre>")
            continue
        if MARKER.match(line.text) or HEADING.match(line.text):
            heading = next(queue)
            out = page.body
            text = inline(heading.text, links, where)
            out.append(f'<h{heading.level} id="{heading.slug}">{text}</h{heading.level}>')
            i += 1
            continue
        if ITEM.match(line.text):
            items: list[str] = []
            while i < len(lines) and ITEM.match(lines[i].text):
                parts = [ITEM.match(lines[i].text).group(1)]
                first = lines[i]
                i += 1
                while i < len(lines) and lines[i].text.strip() and not starts_block(lines[i].text):
                    check_unsupported(lines[i])
                    parts.append(lines[i].text.strip())
                    i += 1
                items.append(f"<li>{inline(chr(10).join(parts), links, f'README.md:{first.number}')}</li>")
            out.append("<ul>\n" + "\n".join(items) + "\n</ul>")
            continue
        check_unsupported(line)
        parts = [line.text.strip()]
        i += 1
        while i < len(lines) and lines[i].text.strip() and not starts_block(lines[i].text):
            check_unsupported(lines[i])
            parts.append(lines[i].text.strip())
            i += 1
        out.append(f"<p>{inline(chr(10).join(parts), links, where)}</p>")
    return page


# --- the page -------------------------------------------------------------------------------


def contents(headings: list[Heading], links: Links) -> str:
    entries = [
        f'<li><a href="#{h.slug}">{inline(h.text, links, "a heading")}</a></li>' for h in headings if h.level == 2
    ]
    return '<nav>\n<h2 id="contents">Contents</h2>\n<ol>\n' + "\n".join(entries) + "\n</ol>\n</nav>"


def build(readme: str, cargo_toml: str) -> str:
    version = workspace_version(cargo_toml)
    url = repo_url(cargo_toml)
    lines = regions(readme)
    headings = headings_of(lines)
    if not any(h.level == 2 for h in headings):
        raise GuideError("README.md: the regions hold no `##` heading, so the page would have no contents")
    links = Links(
        base=f"{url}/blob/v{version}",
        tree=f"{url}/tree/v{version}",
        readme=f"{url}/blob/v{version}/README.md",
        slugs={h.slug for h in headings},
    )
    page = render(lines, links, headings)
    home = html.escape(url)
    shown = html.escape(re.sub(r"^https?://", "", url))
    parts = [
        "<!DOCTYPE html>",
        '<html lang="en">',
        "<head>",
        '<meta charset="utf-8">',
        '<meta name="viewport" content="width=device-width, initial-scale=1">',
        f"<title>mrgl {version} usage guide</title>",
        "<style>",
        STYLE.rstrip("\n"),
        "</style>",
        "</head>",
        "<body>",
        "<header>",
        f'<p class="murloc">{BANNER}</p>',
        f"<h1>mrgl {version}: the usage guide</h1>",
        *page.lead,
        "</header>",
        contents(page.headings, links),
        "<main>",
        *page.body,
        "</main>",
        "<footer>",
        f"<p>The user sections of the README of mrgl {version}, as one page that opens offline next to the exes."
        f' The project, its releases and the rest of its documentation: <a href="{home}">{shown}</a>.</p>',
        "</footer>",
        "</body>",
        "</html>",
    ]
    return "\n".join(parts) + "\n"


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # the README carries em dashes
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--out", type=Path, help="write the page to this file")
    mode.add_argument("--check", action="store_true", help="render the page from README.md, write nothing")
    args = parser.parse_args()

    readme = README.read_text(encoding="utf-8")
    cargo_toml = CARGO_TOML.read_text(encoding="utf-8")
    try:
        page = build(readme, cargo_toml)
        if build(readme, cargo_toml) != page:
            raise GuideError("two renders of the same README differ")
    except GuideError as error:
        print(f"GUIDE.html cannot be rendered: {error}", file=sys.stderr)
        return 1
    sections = page.count("<h2 id=") - 1  # without the contents
    size = len(page.encode("utf-8")) // 1024
    if args.check:
        print(f"GUIDE.html renders from README.md ({sections} sections, {size} KB)")
        return 0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(page, encoding="utf-8", newline="\n")
    print(f"GUIDE.html written: {args.out} ({sections} sections, {size} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
