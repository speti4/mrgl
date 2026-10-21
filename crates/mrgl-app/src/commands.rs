//! The IPC commands the UI calls. Each one is a thin wrapper over `mrgl-core`; the text they return is
//! already in the voice's shape, so the UI prints and never composes a murloc line of its own.

use mrgl_core::VERSION;
use mrgl_core::voice::{Kind, Voice};
use serde::Serialize;

/// A message as the UI shows it: the murloc line (empty when serious) and the plain line.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Message {
    pub murloc: String,
    pub plain: String,
}

impl Message {
    /// Splits the voice's text into its two parts; a serious voice gives an empty murloc line.
    pub fn new(voice: Voice, kind: Kind, plain: impl std::fmt::Display) -> Self {
        let plain = plain.to_string();
        let murloc = if voice.is_serious() {
            String::new()
        } else {
            kind.murloc().to_string()
        };
        Self { murloc, plain }
    }
}

/// The version the status bar shows, from the workspace `Cargo.toml`.
#[tauri::command]
pub fn app_version() -> &'static str {
    VERSION
}

/// The greeting of the empty window; `MRGL_SERIOUS` drops its murloc line as it does in the CLI.
#[tauri::command]
pub fn banner() -> Message {
    Message::new(
        Voice::from_env(false),
        Kind::Banner,
        format!("mrgl {VERSION}: no game folder chosen yet"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_carries_both_lines() {
        let message = Message::new(Voice::new(false), Kind::Success, "2 addons updated");
        assert_eq!(message.murloc, Kind::Success.murloc());
        assert_eq!(message.plain, "2 addons updated");
    }

    #[test]
    fn a_serious_message_has_an_empty_murloc_line() {
        let message = Message::new(Voice::new(true), Kind::Error, "addon not found");
        assert_eq!(message.murloc, "");
        assert_eq!(message.plain, "addon not found");
    }
}
