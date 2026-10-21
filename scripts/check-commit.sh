#!/bin/sh
# The commit rules of docs/git-workflow.md: the subject starts with a type, no Claude-Session trailer
# and no Claude Code session link in a commit message or a pull request text, and the maintainer commits with the
# GitHub noreply address (D-004). Called by .githooks/commit-msg and by the commit-rules job in
# .github/workflows/commit-rules.yml, which also runs the self-test.
#
#   check-commit.sh MESSAGE_FILE [NAME EMAIL]...
#   check-commit.sh --pull-request MESSAGE_FILE
#   check-commit.sh --self-test
#
# Every NAME EMAIL pair is one identity of the commit (author, committer). Exits 1 with a reason per broken rule.
# A commit message loses its `#` comment lines as git's cleanup does; a pull request text (`--pull-request`)
# keeps them, since there a `#` line is a Markdown heading, and its first line is the title, held to the
# subject rule.
set -eu

# The types of docs/git-workflow.md, "Titles": `type: text` or `type(scope): text`, lower case.
types='feat|fix|docs|ci|chore|tasks|deps'

# The maintainer's names: the git config one, and the GitHub login a merge on the web writes.
is_maintainer() { [ "$1" = 'Szőke Péter' ] || [ "$1" = 'speti4' ]; }
maintainer_email='20753030+speti4@users.noreply.github.com'

# Good and bad messages through this script; exits 1 when one gets the wrong verdict.
self_test() {
  dir=$(mktemp -d)
  trap 'rm -rf "$dir"' EXIT
  cases=0
  failures=0
  # check [--pull-request] pass|fail MESSAGE [NAME EMAIL]...
  check() {
    mode=
    if [ "$1" = --pull-request ]; then
      mode=$1
      shift
    fi
    expected=$1
    printf '%s\n' "$2" > "$dir/message.txt"
    shift 2
    cases=$((cases + 1))
    # $mode is unquoted on purpose: empty, it is no argument at all.
    # shellcheck disable=SC2086
    if sh "$0" $mode "$dir/message.txt" "$@" 2> "$dir/stderr.txt"; then actual=pass; else actual=fail; fi
    if [ "$actual" != "$expected" ]; then
      failures=$((failures + 1))
      printf 'self-test: expected %s, got %s: %s\n' "$expected" "$actual" "$(head -n 1 "$dir/message.txt")" >&2
      sed 's/^/  /' "$dir/stderr.txt" >&2
    fi
  }
  trailer='Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>'

  # The subject rule.
  for subject in 'feat: add a second addon' 'fix: keep the flavor id on a multibyte boundary' \
    'docs: describe the search syntax' 'ci: re-run commit-rules on an edited pull request' \
    'chore: polish the code comments' 'tasks: close 001' 'deps: bump serde from 1.0.1 to 1.0.2' \
    'fix(core): skip a broken header' 'feat: add the GitHub release source (task 003) (#12)' \
    "Merge branch 'main' into task/002-flavor-detection-and-addons-scan-with-toc-parsing" \
    'Merge pull request #12 from someone/a-branch' \
    'Revert "feat: add a second addon"'; do
    check pass "$subject

$trailer"
  done
  for subject in 'add a second addon' 'Feat: add a second addon' 'FIX: a bug' 'feature: add a second addon' \
    'feat:add a second addon' 'feat : add a second addon' 'feat():  add a second addon' 'feat(): add a second addon' \
    'feat:' 'feat: ' 'docs, tasks: two types' 'fixup! feat: add a second addon' 'Revert feat: add a second addon' \
    'Merged the branch' 'WIP'; do
    check fail "$subject

$trailer"
  done
  check pass "

feat: a subject after blank lines"
  check pass "# a comment line git strips
feat: a subject below a comment"
  check fail ''

  # The D-004 rules: trailer, link, author address.
  check fail "feat: a subject

Claude-Session: https://example.invalid/session"
  check fail "feat: a subject

See https://claude.ai/code/session_0123456789abcdef"
  check pass "feat: a subject" 'Szőke Péter' "$maintainer_email" 'Szőke Péter' "$maintainer_email"
  check fail "feat: a subject" 'Szőke Péter' 'someone@example.com' 'Szőke Péter' "$maintainer_email"
  check pass "feat: a subject" 'A Contributor' 'contributor@example.com' 'GitHub' 'noreply@github.com'

  # A pull request text keeps its Markdown headings, so a rule broken on a heading line still counts.
  check --pull-request pass "feat: a subject

## What and why

A body under a heading."
  check --pull-request fail "feat: a subject

## See https://claude.ai/code/session_0123456789abcdef"
  check --pull-request fail "# not a subject

feat: a subject below a heading"

  if [ "$failures" -ne 0 ]; then
    echo "self-test: $failures of $cases cases failed" >&2
    exit 1
  fi
  echo "self-test: $cases cases passed"
  exit 0
}

if [ "${1-}" = --self-test ]; then
  self_test
fi

pull_request=
if [ "${1-}" = --pull-request ]; then
  pull_request=1
  shift
fi
message_file=$1
shift
status=0

# In a commit message, comment lines and everything below the scissors line of `git commit -v` (the diff) are
# not the message; a pull request text is taken as it is.
if [ -n "$pull_request" ]; then
  message=$(cat "$message_file")
else
  message=$(sed -e '/^# -* >8 -*$/,$d' -e '/^#/d' "$message_file")
fi

# The subject is the first line that is not blank, as git's cleanup keeps it. A merge or a revert subject passes
# as git writes it.
subject=$(printf '%s\n' "$message" | sed -n '/[^[:space:]]/{p;q;}')
case $subject in
  'Merge '* | 'Revert "'*) ;;
  *)
    if ! printf '%s\n' "$subject" | grep -Eq "^($types)(\([^()[:space:]]+\))?: [^[:space:]]"; then
      echo "commit rules: the subject starts with a type and a colon, \`feat: …\` or \`feat(scope): …\`, one of" \
        "$(echo "$types" | sed 's/|/ /g') (docs/git-workflow.md): ${subject:-(empty)}" >&2
      status=1
    fi
    ;;
esac

if printf '%s\n' "$message" | grep -qi '^claude-session:'; then
  echo "commit rules: no Claude-Session: trailer (D-004); Co-Authored-By is enough" >&2
  status=1
fi
if printf '%s\n' "$message" | grep -q 'claude\.ai/code/session_'; then
  echo "commit rules: no Claude Code session link in a commit or pull request text (D-004)" >&2
  status=1
fi

reported=
while [ $# -ge 2 ]; do
  if is_maintainer "$1" && [ "$2" != "$maintainer_email" ] && [ "$2" != "$reported" ]; then
    echo "commit rules: $1 commits as $maintainer_email, not $2 (D-004)" >&2
    reported=$2
    status=1
  fi
  shift 2
done

exit $status
