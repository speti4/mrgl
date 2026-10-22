---
status: decided
date: 2026-10-22
cost: medium
sources: [backlog/031-self-update-with-attestation-verification.md, backlog/103-authenticode-signed-exes-through-signpath.md, D-002-rust-tauri2-portable-zip-cf-aware.md]
---

# D-006 — Self-update trusts the Authenticode signature and the checksum; the Sigstore attestation stays a `gh` check for humans

## Context

A release carries the portable zip, `SHA256SUMS.txt` and a GitHub build-provenance attestation, a Sigstore
bundle (a DSSE signature, a Fulcio certificate, a Rekor log entry) that a person verifies with
`gh attestation verify <zip> --repo speti4/mrgl`. Task 031 wants `mrgl self-update` to verify the downloaded
zip itself, and the question was how the exe verifies the attestation without `gh`: a full Sigstore
verification needs a large dependency tree and a trusted root that rotates with the releases; a reduced
verification (the signature and the certificate against a pinned Fulcio root, no Rekor) is trust logic the
project would have to keep right by hand; the checksum alone comes from the same place as the zip and protects
against a corrupt download only. The roadmap's M5 signs the two exes through SignPath (Authenticode), and
Windows verifies an Authenticode signature itself.

## Decision

The self-update of the portable zip trusts two things: the `SHA256SUMS.txt` of the release for the download,
and the Authenticode signature of each exe in the zip, verified through the Windows `WinVerifyTrust` API
(`windows-sys`, no new crate, D-002) against the pinned subject of the SignPath-issued certificate. The
Sigstore attestation of the zip stays what it is today, a check a person runs with `gh attestation verify`;
no Sigstore crate and no hand-written certificate logic enter the tree. Consequently the self-update (031)
waits for the signed exes (103), and the macOS preview (M6) needs its own signature check when it comes.
Decided 2026-10-22.

## Not chosen

- A full Sigstore verification in Rust (the `sigstore` crate): the dependency tree and the rotating trusted
  root outweigh what it adds over a signature Windows already verifies.
- A reduced verification of the bundle (the DSSE signature and the certificate against a pinned Fulcio root,
  no transparency log): hand-kept trust logic, and a pinned root that rotates.
- The checksum alone: it only detects a corrupt download, which is not trust.
- Delegating to an installed `gh`: the zip has nothing to install (D-002), and a check that depends on another
  tool is not the tool's own.
