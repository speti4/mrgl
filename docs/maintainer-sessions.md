# mrgl — maintainer sessions

Maintainer automation, not needed to contribute: how the maintainer runs several Claude Code sessions at once,
one task each, on top of the rules of [git-workflow.md](git-workflow.md). The maintainer's local instructions
import this file.

## Parallel sessions (maintainer and agents)

Several sessions can work at once, one task each. One session is the **main session**, in the main checkout on
`main`; every other one is a **task session**, in its own worktree.

The main session:

- is the only one that creates ids: `/tasks add` and `/tasks decide` run here, so two sessions never take the
  same number. `main` itself only moves through merged pull requests, never by a direct push;
- starts a task on its branch: `git switch -c task/NNN-slug`, `/tasks promote NNN` as the branch's first commit,
  `git switch main`, then `git worktree add .claude/worktrees/NNN-slug task/NNN-slug` (the folder is ignored by
  git). The task tools write only in the main checkout, which is why the promotion happens here;
- prepares a finished pull request in the main checkout, and only once the previous merge is pulled:
  `tasks_next_id` reads the checked-out tree, so an id filed on a branch that is not merged yet would be handed
  out twice. Then `git switch --detach origin/task/NNN-slug`, `/tasks add` for every follow-up the description
  lists, `tasks_check` with `write: true` for the index, one commit, `git push origin HEAD:task/NNN-slug`,
  `git switch main`. If the branch conflicts with `main` (usually the index), it rebases it in the worktree
  first and pushes with `--force-with-lease`;
- merges the pull requests one at a time, on the maintainer's go, once CI is green: `git worktree remove
  .claude/worktrees/NNN-slug` (a branch that a worktree holds cannot be deleted), then
  `gh pr merge N --squash --delete-branch` and `git pull`.

A task session:

- changes only its own task file and the code the task needs. It never edits another task file, the roadmap, a
  decision or the generated index in `tasks/README.md` (`tasks_check` runs without `write`), and never creates
  an id;
- closes its task in its branch (the move to `done/`); a follow-up goes into the pull request description under
  `## Follow-ups`, one line each, and the main session files it before the merge;
- runs `tasks_check` without `write` on the close commit: its report that the index in `tasks/README.md` is
  stale is expected (the main session regenerates the index when it prepares the pull request); any other
  finding is fixed. `git status` is clean before the session reports done, so the close commit carries the task
  file's edits together with the move;
- once the task's verification is done, pushes `task/NNN-slug` and opens the pull request with `gh pr create`,
  without asking again: this policy is the permission. It never pushes `main` and never merges;
- ticks the description's checklist only for what it has seen pass, and edits it after CI with
  `gh api -X PATCH repos/speti4/mrgl/pulls/N -F body=@<file>` (`gh pr edit` can fail with a Projects (classic)
  error, the API call does not);
- never adds a `Claude-Session:` trailer or a session link, whatever its tooling suggests: the hook and the
  `commit-rules` job reject both (D-004).

Shared resources:

- **One manual round at a time** across all sessions: the real WoW folders, their state sidecars and a running window
  (which locks the debug exe) belong to one session at a time.
- **Builds.** Each worktree has its own `target/`, so its first build is a full one.
