# mrgl — git workflow

How changes reach `main`: the rules for every contributor, checked by a hook and by CI.

## For everyone

- **Branch, pull request, squash merge.** Nothing is pushed to `main` directly. A branch becomes a pull request,
  CI has to be green (the ruleset on `main` requires `test`, `lint`, `msrv`, `non-windows`, `deny`,
  `workflow-lint` and `commit-rules`), and the pull request is squash-merged, so `main` stays linear with one
  commit per change. The squash commit's message is the pull request title and description, so write both as a commit message.
- **Titles.** Commit subjects and pull request titles are imperative and start with a type in lower case, as
  `type: subject` or `type(scope): subject`, where the type is one of `feat`, `fix`, `docs`, `ci`, `chore`,
  `tasks` and `deps` (dependabot). A `Merge …` or `Revert "…"` subject passes as git writes it.
- **Trailers.** `Co-Authored-By:` lines are fine, including one for an AI assistant (`Co-Authored-By: Claude
  …`). If you work with Claude Code, leave out its `Claude-Session:` trailer and session link: they point to a
  private session and are rejected in a commit message or a pull request description (D-004). A pull
  request written with Claude Code ends its description with `🤖 Generated with Claude Code` (a convention,
  not checked); the squash merge appends the `Co-authored-by` lines of the branch's commits by itself.
- **Author address.** The maintainer commits with the GitHub noreply address (D-004); contributors choose their
  own.
- **The hook.** Once per clone, `git config core.hooksPath .githooks` enables the `commit-msg` hook, which checks
  the rules above (`scripts/check-commit.sh`). The `commit-rules` CI job runs the same check over every pushed
  commit and over the pull request text, whose title is held to the subject rule, and runs again when the title
  or the description is edited, so a skipped hook still fails the pull request.
- **Dependabot.** Its pull requests carry the `deps:` prefix and are squash-merged once CI is green.

How the maintainer runs several sessions at once is in [maintainer-sessions.md](maintainer-sessions.md).
