//! Addon sources, in the order D-002 names them: GitHub releases with `release.json` (task 003), then Wago,
//! WoWInterface and CurseForge with the user's own key (M4 Sources), and a zip in hand. A source answers two
//! questions, "which releases exist for this addon" and "which asset is the one for this flavor", and never
//! guesses "latest" without saying so. Network access is behind a trait so the tests run offline.
