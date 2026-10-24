"""Tests of scripts/usage-guide.py: one test per README the generator refuses, and the page it renders from a
small fixture README.

The standard library only (D-002). The script's name has a hyphen, so it is loaded from its path with importlib
rather than imported by name. Every fixture README is a string built here, never a copy of the real README; the
README and Cargo.toml the script reads are swapped for temporary files, and `main()` runs as `--check` does, so
each rejection test sees the exit code and the message the lint job would print. A link in a fixture names a file
of the real repo, since the generator checks a link against the checkout it sits in.

    python -I -m unittest discover -s scripts/tests     # from the root of the repo
"""

from __future__ import annotations

import importlib.util
import io
import re
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).resolve().parent.parent / "usage-guide.py"


def load_script():
    spec = importlib.util.spec_from_file_location("usage_guide", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module  # the dataclasses look their module up by name
    spec.loader.exec_module(module)
    return module


guide = load_script()

REPO = "https://example.invalid/mrgl"
CARGO_TOML = f'[workspace.package]\nversion = "9.8.7"\nrepository = "{REPO}"\n'

# A region with one heading: a line handed to page() lands on README line FIRST + its index.
FIRST = 4


def page(*body: str) -> str:
    return "\n".join(["<!-- guide: start -->", "## Usage", "", *body, "<!-- guide: end -->", ""])


def check(readme: str, cargo_toml: str = CARGO_TOML) -> tuple[int, str, str]:
    """Runs the script's `main()` as `--check` over this README: the exit code, stdout and stderr."""
    with tempfile.TemporaryDirectory() as folder:
        readme_path = Path(folder) / "README.md"
        cargo_path = Path(folder) / "Cargo.toml"
        readme_path.write_text(readme, encoding="utf-8")
        cargo_path.write_text(cargo_toml, encoding="utf-8")
        out = io.TextIOWrapper(io.BytesIO(), encoding="utf-8")  # main() reconfigures stdout
        err = io.StringIO()
        with (
            mock.patch.object(guide, "README", readme_path),
            mock.patch.object(guide, "CARGO_TOML", cargo_path),
            mock.patch.object(sys, "argv", ["usage-guide.py", "--check"]),
            redirect_stdout(out),
            redirect_stderr(err),
        ):
            code = guide.main()
        out.flush()
        return code, out.buffer.getvalue().decode("utf-8"), err.getvalue()


class Rejections(unittest.TestCase):
    """Each README here stops `--check` with exit 1, and the message names the README line where the generator
    knows one (a marker, a heading, a block, an inline construct, a link) and the README alone where it does not
    (a region or a fence left open, no marker, no `##` heading)."""

    def assert_rejected(self, readme: str, where: str, what: str) -> None:
        code, out, err = check(readme)
        self.assertEqual(code, 1, f"accepted: {out!r}")
        self.assertEqual(out, "")
        prefix = f"GUIDE.html cannot be rendered: {where}: "
        self.assertTrue(err.startswith(prefix), f"{err!r} does not start with {prefix!r}")
        self.assertIn(what, err)

    def assert_line(self, readme: str, line: int, what: str) -> None:
        self.assert_rejected(readme, f"README.md:{line}", what)

    # --- the markers

    def test_marker_the_page_cannot_read(self):
        self.assert_line(page("Text.", "<!-- guide: section Later"), FIRST + 1, "a guide marker the page cannot read")

    def test_unknown_marker(self):
        self.assert_line(page("<!-- guide: begin -->"), FIRST, "an unknown guide marker")

    def test_section_marker_without_a_title(self):
        self.assert_line(page("<!-- guide: section -->"), FIRST, "an unknown guide marker")

    def test_start_inside_a_region(self):
        self.assert_line(page("Text.", "<!-- guide: start -->"), FIRST + 1, "`guide: start` inside a region")

    def test_end_without_a_start(self):
        self.assert_line("Intro.\n<!-- guide: end -->\n", 2, "`guide: end` without a start")

    def test_section_outside_a_region(self):
        readme = page("Text.") + "<!-- guide: section Later -->\n"
        self.assert_line(readme, FIRST + 2, "`guide: section` outside a region")

    def test_start_without_its_end(self):
        self.assert_rejected("<!-- guide: start -->\n## Usage\n\nText.\n", "README.md", "without its end")

    def test_code_fence_without_its_end(self):
        # A fence left open inside a region turns its end marker into code, so that README fails as a region
        # without its end; the fence's own message is for one left open after the last region.
        self.assert_rejected(page("Text.") + "```\ncode\n", "README.md", "a code fence without its end")
        self.assert_rejected(page("```", "code"), "README.md", "a `guide: start` without its end")

    def test_no_start_marker(self):
        self.assert_rejected("## Usage\n\nText.\n", "README.md", "no `<!-- guide: start -->` marker")

    def test_no_level_two_heading(self):
        readme = "<!-- guide: start -->\n### Usage\n\nText.\n<!-- guide: end -->\n"
        self.assert_rejected(readme, "README.md", "no `##` heading")

    # --- the headings and the blocks

    def test_heading_level_the_page_does_not_read(self):
        self.assert_line(page("#### Detail"), FIRST, "a level-4 heading")

    def test_table(self):
        self.assert_line(page("| a | b |", "| - | - |"), FIRST, "a table")

    def test_block_quote(self):
        self.assert_line(page("> Quoted."), FIRST, "a block quote")

    def test_numbered_list(self):
        self.assert_line(page("1. First"), FIRST, "a numbered list")

    def test_list_item_with_a_star(self):
        self.assert_line(page("* Item"), FIRST, "a list item with `*` or `+`")

    def test_list_item_with_a_plus(self):
        self.assert_line(page("+ Item"), FIRST, "a list item with `*` or `+`")

    def test_nested_list(self):
        self.assert_line(page("- Item", "  - Nested"), FIRST + 1, "a nested list")

    def test_indented_code_block(self):
        self.assert_line(page("    indented code"), FIRST, "an indented code block")

    def test_thematic_break(self):
        self.assert_line(page("Text.", "", "---"), FIRST + 2, "a thematic break or a setext heading")

    def test_setext_heading(self):
        self.assert_line(page("Title", "====="), FIRST + 1, "a thematic break or a setext heading")

    # The block check names the line itself ("raw HTML: <line>"), the inline check would catch the same line
    # as "raw HTML or an autolink in ..."; the message tells which one stopped it.
    def test_raw_html_block(self):
        self.assert_line(page("<div>Block</div>"), FIRST, "raw HTML: <div>Block</div>")

    def test_other_html_comment(self):
        self.assert_line(page("<!-- a note -->"), FIRST, "raw HTML: <!-- a note -->")

    def test_construct_on_a_continuation_line(self):
        self.assert_line(page("Text that goes on", "> quoted."), FIRST + 1, "a block quote")

    # --- the inline Markdown: the message names the first line of the paragraph or the list item

    def test_image(self):
        self.assert_line(page("Text ![logo](README.md) here."), FIRST, "an image")

    def test_image_without_a_link_target(self):
        self.assert_line(page("Text ![logo] here."), FIRST, "an image")

    def test_emphasis_with_a_star(self):
        self.assert_line(page("Text", "with *stress*."), FIRST, "emphasis with `*`")

    def test_emphasis_with_an_underscore(self):
        self.assert_line(page("Text with _stress_."), FIRST, "emphasis with `_`")

    def test_inline_raw_html(self):
        self.assert_line(page("Text with <span>markup</span>."), FIRST, "raw HTML or an autolink")

    def test_autolink(self):
        self.assert_line(page("See <https://example.invalid>."), FIRST, "raw HTML or an autolink")

    def test_reference_link(self):
        self.assert_line(page("See [the rules][rules]."), FIRST, "a link the page cannot read")

    def test_construct_in_a_list_item(self):
        self.assert_line(page("Text.", "", "- Item", "  with *stress*."), FIRST + 2, "emphasis with `*`")

    def test_construct_in_a_heading(self):
        self.assert_line(page("## A *stressed* heading"), FIRST, "emphasis with `*`")

    # --- the links

    def test_link_to_a_missing_file(self):
        self.assert_line(page("See [it](no-such-file.md)."), FIRST, "the link `no-such-file.md` names no file")

    def test_link_out_of_the_repo(self):
        self.assert_line(page("See [it](../README.md)."), FIRST, "the link `../README.md` names no file")

    # --- the inputs and the render

    def test_cargo_toml_without_a_version(self):
        code, _, err = check(page("Text."), f'[workspace.package]\nrepository = "{REPO}"\n')
        self.assertEqual(code, 1)
        self.assertIn("Cargo.toml: no [workspace.package] version", err)

    def test_cargo_toml_without_a_repository(self):
        code, _, err = check(page("Text."), '[workspace.package]\nversion = "9.8.7"\n')
        self.assertEqual(code, 1)
        self.assertIn("Cargo.toml: no `repository`", err)

    def test_render_that_is_not_deterministic(self):
        with mock.patch.object(guide, "build", side_effect=["<p>one</p>\n", "<p>two</p>\n"]):
            code, out, err = check(page("Text."))
        self.assertEqual(code, 1)
        self.assertEqual(out, "")
        self.assertIn("two renders of the same README differ", err)


FIXTURE = """\
# mrgl

Outside the regions: not on the page.

<!-- guide: start -->
The intro, with **bold**, a `code span`, an escaped \\*star\\* and a snake_case_name.

## Install

Unpack the zip, then run `target\\release\\mrgl.exe --version`.

- The rules: [contributing](CONTRIBUTING.md)
  and the [docs folder](docs/).
- A page anchor: [the commands](#the-commands), a README anchor: [elsewhere](#not-on-the-page).

```powershell
target\\release\\mrgl.exe scan
<!-- guide: end -->
```
<!-- guide: end -->

## Build from source

Not on the page either.

<!-- guide: start -->
<!-- guide: section The commands -->
Prose under a heading the page leaves out.

### One command

A link out: [the web](https://example.invalid/page).
<!-- guide: end -->
"""


class HappyPath(unittest.TestCase):
    def setUp(self):
        self.html = guide.build(FIXTURE, CARGO_TOML)

    def test_check_passes(self):
        code, out, err = check(FIXTURE)
        self.assertEqual((code, err), (0, ""))
        self.assertTrue(out.startswith("GUIDE.html renders from README.md (2 sections, "), out)

    def test_deterministic(self):
        self.assertEqual(guide.build(FIXTURE, CARGO_TOML), self.html)

    def test_sections_and_contents(self):
        self.assertEqual(
            re.findall(r'<h([23]) id="([^"]+)">', self.html),
            [("2", "contents"), ("2", "install"), ("2", "the-commands"), ("3", "one-command")],
        )
        self.assertIn(
            '<li><a href="#install">Install</a></li>\n<li><a href="#the-commands">The commands</a></li>', self.html
        )
        self.assertIn("<title>mrgl 9.8.7 usage guide</title>", self.html)
        self.assertIn('<p class="murloc">Mrglglgl.</p>', self.html)

    def test_only_the_regions(self):
        self.assertNotIn("Outside the regions", self.html)
        self.assertNotIn("Not on the page either", self.html)
        self.assertNotIn("Build from source", self.html)
        lead = self.html.split("<header>", 1)[1].split("</header>", 1)[0]
        self.assertIn("The intro, with <strong>bold</strong>, a <code>code span</code>", lead)
        self.assertIn("an escaped *star* and a snake_case_name.", lead)

    def test_inline_and_blocks(self):
        self.assertIn("<code>.\\mrgl.exe --version</code>", self.html)
        self.assertIn("<pre><code>.\\mrgl.exe scan\n&lt;!-- guide: end --&gt;</code></pre>", self.html)
        self.assertNotIn("target\\release\\", self.html)
        self.assertEqual(self.html.count("<li>"), 2 + 2)  # the list and the contents

    def test_links(self):
        hrefs = re.findall(r'href="([^"]*)"', self.html)
        for href in [
            f"{REPO}/blob/v9.8.7/CONTRIBUTING.md",
            f"{REPO}/tree/v9.8.7/docs",
            "#the-commands",
            f"{REPO}/blob/v9.8.7/README.md#not-on-the-page",
            "https://example.invalid/page",
        ]:
            self.assertIn(href, hrefs)
        for href in hrefs:
            self.assertTrue(href.startswith(("#", REPO, "https://example.invalid/")), href)

    def test_no_external_asset(self):
        for asset in ("<script", "src=", "<link", "url(", "@import", "<img"):
            self.assertNotIn(asset, self.html)


if __name__ == "__main__":
    unittest.main()
