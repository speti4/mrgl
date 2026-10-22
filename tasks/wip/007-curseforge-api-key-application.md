---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
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

1. [ ] The research page: the application route, the terms (distribution, no caching, no circumvention, the
       key's confidentiality, the quota, termination), what they mean for the source implementation
2. [ ] The application submitted on the form, with the texts of the research page; the date in the Log
3. [ ] The outcome (a key issued, with its terms, or refused, with the reason) on the research page; close

## Log

- 2026-10-22 — promoted; runs in the main checkout on its own branch, guided step by step

## Deviations from the plan

- none yet

## Verification

- (dated: the application's date, the outcome's date, the research page's sources)

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
