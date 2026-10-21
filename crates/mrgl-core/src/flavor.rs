//! A game flavor folder's identity (task 002 fills this in). The flavor is read from the folder itself, never
//! guessed from its name: `.flavor.info` holds `wow`, `wow_classic`, `wow_classic_era` or `wowf`, and
//! `.build.info` the version and product. That value is also the state sidecar's folder name (D-003).

/// The flavor ids Blizzard's launcher writes into `.flavor.info`, with the folder they usually live in.
pub const KNOWN_FLAVORS: [(&str, &str); 4] = [
    ("wow", "_retail_"),
    ("wow_classic", "_classic_"),
    ("wow_classic_era", "_classic_era_"),
    ("wowf", "_forever_"),
];
