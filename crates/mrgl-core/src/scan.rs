//! The AddOns scan (task 002): every folder under `Interface\AddOns` with a TOC becomes an installed addon,
//! with its TOC data ([`crate::toc`]), its source hints (`X-Curse-Project-ID`, `X-Wago-ID`, `X-WoWI-ID`) and
//! whether the state sidecar knows it. The scan never writes.
