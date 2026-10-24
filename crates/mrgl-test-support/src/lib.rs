//! mrgl-test-support: the helpers the tests of `mrgl-core`, `mrgl-cli` and `mrgl-app` share (task 118), a
//! dev-dependency of the three crates and never part of an exe. [`stand_in`] is the stand-in game client of
//! the running-client tests (tasks 023 and 063), [`locked`] the file and folder handles the tests hold open
//! so that a read, a move or a removal under them fails. Windows only, as the tests that use them; the
//! other targets compile an empty crate.

// The `coverage(off)` attribute below (docs/development.md, "Coverage"); the feature gate exists only under
// cargo-llvm-cov's nightly cfg, so the stable build never sees it.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// coverage: off — test code the tests of the three crates share, not the subject of the measurement, like a
// `#[cfg(test)]` module
#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(windows)]
pub mod stand_in;

// coverage: off — test code the tests of the three crates share, not the subject of the measurement, like a
// `#[cfg(test)]` module
#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(windows)]
pub mod locked;
