//! Every setting of the app, edited in the Settings view (⌘,) and nowhere else: no
//! configuration file to edit by hand, no environment variable, no hidden flag.
//! Adding one means a field here, with its default, and its entry in
//! `src/lib/Settings.svelte`, and for a number, its range in `BOUNDS`. The Linear API key is the one exception kept out of this
//! file: it lives in the macOS keychain, and Settings only sets it.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::store;

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

impl Settings {
    /// The settings with each number brought within its range in `BOUNDS`.
    #[must_use]
    pub fn clamped(self) -> Self {
        let b = &BOUNDS;
        Self {
            search_limit: b.search_limit.clamp(self.search_limit),
            refresh_minutes: b.refresh_minutes.clamp(self.refresh_minutes),
            session_seconds: b.session_seconds.clamp(self.session_seconds),
            stats_days: b.stats_days.clamp(self.stats_days),
            backlog_done: b.backlog_done.clamp(self.backlog_done),
            ..self
        }
    }

    /// The settings saved at `path`, within their ranges: see `store::load`.
    pub fn load(path: &Path) -> Self {
        store::load::<Self>(path).clamped()
    }
}

/// The range a number setting may take, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Range {
    pub min: u32,
    pub max: u32,
}

impl Range {
    const fn new(min: u32, max: u32) -> Self {
        Self { min, max }
    }

    fn clamp(self, value: u32) -> u32 {
        value.clamp(self.min, self.max)
    }
}

/// The range of each number setting, by field of `Settings`. Kept here only: the
/// Settings view takes its limits from the `Snapshot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Bounds {
    pub search_limit: Range,
    pub refresh_minutes: Range,
    pub session_seconds: Range,
    pub stats_days: Range,
    pub backlog_done: Range,
}

pub const BOUNDS: Bounds = Bounds {
    search_limit: Range::new(5, 500),
    refresh_minutes: Range::new(1, 60),
    session_seconds: Range::new(5, 120),
    stats_days: Range::new(7, 365),
    backlog_done: Range::new(0, 500),
};

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

    /// Settings with every number set to `value`.
    fn all(value: u32) -> Settings {
        Settings {
            search_limit: value,
            refresh_minutes: value,
            session_seconds: value,
            stats_days: value,
            backlog_done: value,
            ..Settings::default()
        }
    }

    #[test]
    fn the_defaults_are_within_their_bounds() {
        assert_eq!(Settings::default().clamped(), Settings::default());
    }

    #[test]
    fn a_number_out_of_its_range_is_brought_to_the_nearest_end() {
        let b = BOUNDS;
        let low = all(0).clamped();
        assert_eq!(
            [
                low.search_limit,
                low.refresh_minutes,
                low.session_seconds,
                low.stats_days,
                low.backlog_done
            ],
            [
                b.search_limit.min,
                b.refresh_minutes.min,
                b.session_seconds.min,
                b.stats_days.min,
                b.backlog_done.min
            ],
        );
        let high = all(u32::MAX).clamped();
        assert_eq!(
            [
                high.search_limit,
                high.refresh_minutes,
                high.session_seconds,
                high.stats_days,
                high.backlog_done
            ],
            [
                b.search_limit.max,
                b.refresh_minutes.max,
                b.session_seconds.max,
                b.stats_days.max,
                b.backlog_done.max
            ],
        );
        assert_eq!(all(30).clamped(), all(30));
    }

    #[test]
    fn settings_loaded_from_disk_are_within_their_bounds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"search_limit": 0, "stats_days": 10000, "refresh_minutes": 30}"#,
        )
        .unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.search_limit, BOUNDS.search_limit.min);
        assert_eq!(settings.stats_days, BOUNDS.stats_days.max);
        assert_eq!(settings.refresh_minutes, 30);
    }
}
