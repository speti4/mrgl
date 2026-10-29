---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-29
plan: none
milestone: M4 Sources
---

# 007 — Apply for a CurseForge third-party API key

## Context

Submit the application on the CurseForge developer console for an API key for mrgl (name, purpose, the
terms: the user's own key, `allowModDistribution` honored, no scraping), and record the outcome and the
terms that apply in `docs/research/`. The CurseForge source of M4 needs a key, the approval is manual, and
the application does not depend on any code, so it is independent of M1–M3.
Done when: a key is issued or refused, and the research page states the terms the source implementation has
to follow.

Decided 2026-10-22: the application goes through the form the API documentation links ("apply for a key",
reviewed by Overwolf, the key sent by e-mail and shown in the developer console); the research page
`docs/research/2026-10-22-curseforge-api-terms.md` is written at the application, the outcome is added when
it arrives. Since 2026-07-16 a download from the CurseForge CDN needs a key as well (the x-api-key header
or an api-key query parameter), so without a key the source of M4 can neither resolve nor download.

## Docs that move

- `docs/research/2026-10-22-curseforge-api-terms.md` (new): the application, the terms the source
  implementation follows, the outcome.
- `docs/research/2026-10-21-addon-manager-alternatives.md` ("CurseForge"): a link to the new page.

## Not in this change

- The CurseForge source itself (M4); the key's storage and the doctor's key line (056); any code.

## Steps

1. [x] The research page: the application route, the terms (distribution, no caching, no circumvention, the
       key's confidentiality, the quota, termination), what they mean for the source implementation
2. [x] The application submitted on the form, with the texts of the research page; the date in the Log
3. [x] The outcome (a key issued, with its terms, or refused, with the reason) on the research page; close

## Log

- 2026-10-22 — promoted; runs in the main checkout on its own branch, guided step by step
- 2026-10-22 — the research page written from the API documentation, the support articles (the application
  article and the 3rd Party API Terms) and the 2026-06-10 post on key authentication for CDN downloads
- 2026-10-22 — the application submitted on the "CurseForge API Applications" form (project name `mrgl`,
  game WoW, the repository as website and git URL, no monetization, the texts of the research page: Rust,
  one portable zip, the user's own key, `allowModDistribution` honored, no scraping, no caching, no
  distribution; the author-approval, volume and terms boxes ticked). The form answered "Thank you"; the
  developer console already holds an organization key of the Studios kind, which is not the one applied for.
  Waiting for Overwolf's review, which is sent by e-mail; no review time is published.
- 2026-10-29 — approved: the e-mail "Your CurseForge API Application - Approved" (seven days after the
  application) names the application `mrgl` and the 3rd Party CurseForge API and carries the personal key.
  The outcome is on the research page; the key went into the Windows Credential Manager through `mrgl key
  set curse` from the clipboard (never an argument, never a file in the repo), and `mrgl key list` and the
  doctor report `MRGL_CURSEFORGE_KEY set (credential store)`. The first real-API requests are task 096's
  step 5. No follow-up: the application was the whole task.

## Deviations from the plan

- The branch waited locally for the answer instead of being merged as "submitted": one pull request carries
  the application and its outcome.

## Verification

- 2026-10-22 — the application submitted; the research page's sources are listed on the page.
- 2026-10-29 — the key is issued and stored: `mrgl key list` reports `MRGL_CURSEFORGE_KEY set (credential
  store)` and the doctor's `api keys:` line the same, with no key value in either output; the research page
  states the outcome and the terms the source implementation follows (its table, unchanged by the approval).

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log (none)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: 2026-10-29`
