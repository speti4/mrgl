---
status: idea
size: M
origin: session 2026-10-22
created: 2026-10-22
milestone: M5 Trust
---

# Authenticode-signed exes through SignPath

What: the release job signs `mrgl.exe` and `mrgl-app.exe` through SignPath's open-source program before the
zip is built, the release notes and the README name the signer, and the subject of the certificate is
recorded in the repo for the self-update's check (D-006); the application to SignPath comes first and the
task waits on its answer, as 007 did for CurseForge.
Why: a signed exe is what Windows and the self-update (031) can verify on their own; the roadmap's M5 asks
for two signed exes in every release.
Done when: a release carries two exes whose signature `Get-AuthenticodeSignature` reports valid with the
recorded subject, and the release workflow fails when the signing step does.
