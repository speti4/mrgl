---
status: idea
size: M
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
milestone: M3 Toolbox
---

# Addon sets: undo an apply, edit a stored set, `--account`, and uninstalled addons on save

What: `set apply` keeps the `AddOns.txt` files it replaced so `set undo` puts them back; `set enable|disable|drop
<set> <addon>` edits a stored set without the files; `--account` picks a character when one realm and name
sit under two account folders (027); `set save` warns about addons that are no longer installed, or keeps
only installed ones (decided in the task).
Why: a wrong apply has no way back today, and a set can only change by saving it again from the files.
Done when: a fixture apply round-trips through undo byte for byte, an edit changes only the named addon, and
the two-account case resolves with `--account`.
