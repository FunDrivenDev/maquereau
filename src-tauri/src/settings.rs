//! Every setting of the app, edited in the Settings view (⌘,) and nowhere else: no
//! configuration file to edit by hand, no environment variable, no hidden flag.
//! Adding one means a field here, with its default, and its entry in
//! `src/lib/Settings.svelte`. The Linear API key is the one exception kept out of this
//! file: it lives in the macOS keychain, and Settings only sets it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    /// How many results a search shows at most.
    pub search_limit: u32,
    /// The terminal a topic's herdr session opens in.
    pub terminal: Terminal,
    /// The folder a new topic's session starts in.
    pub folder: String,
    /// Minutes between two reads of Linear and GitHub.
    pub refresh_minutes: u32,
    /// Seconds between two reads of the herdr sessions.
    pub session_seconds: u32,
    /// Notify when an agent of a topic's session needs an answer or has finished.
    pub notify_sessions: bool,
    /// Notify when someone else comments on a linked issue or pull request.
    pub notify_comments: bool,
    /// How many days of done topics the flow stats count.
    pub stats_days: u32,
    /// How many done topics the backlog lists, the most recently finished first.
    pub backlog_done: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            search_limit: 50,
            terminal: Terminal::Ghostty,
            folder: "~/Code".into(),
            refresh_minutes: 5,
            session_seconds: 15,
            notify_sessions: true,
            notify_comments: true,
            stats_days: 90,
            backlog_done: 30,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Terminal {
    #[default]
    Ghostty,
    Terminal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_unknown_field_keeps_its_default() {
        let settings: Settings = serde_json::from_str(r#"{"theme": "dark", "gone": 1}"#).unwrap();
        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(settings.search_limit, Settings::default().search_limit);
        assert_eq!(settings.terminal, Terminal::Ghostty);
    }
}
