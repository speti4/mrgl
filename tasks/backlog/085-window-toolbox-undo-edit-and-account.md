---
status: idea
size: M
origin: done/074-window-toolbox-sets-and-savedvariables.md
created: 2026-10-22
milestone: M3 Toolbox
---

# The window's toolbox undoes an apply, edits a set and names the account

What: the toolbox tab (074) offers the undo of the last apply (`sets::undo_apply`), the enable, disable and
drop of one addon in a stored set (`sets::edit_set`), and passes an account through
`sets::apply_set_in_account` when one realm and name sit under two accounts (073), each refused while the
client runs as the apply is.
Why: 073 landed the undo, the edit and `--account` on the command line in the same wave as the window's
set switch, so the window applies sets but cannot take one back or change one.
Done when: on the fixture flavor the window undoes an apply byte for byte, an edit changes only the named
addon in `sets.json`, and the two-account case applies to the chosen account.
