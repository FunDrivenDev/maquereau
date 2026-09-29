//! Every setting of the app, edited in the Settings view (⌘,) and nowhere else: no
//! configuration file to edit by hand, no environment variable, no hidden flag.
//! Adding one means a field here, with its default, and its entry in
//! `src/lib/Settings.svelte`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    /// How many results a search shows at most.
    pub search_limit: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            search_limit: 50,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_unknown_field_keeps_its_default() {
        let settings: Settings = serde_json::from_str(r#"{"theme": "dark", "gone": 1}"#).unwrap();
        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(settings.search_limit, Settings::default().search_limit);
    }
}
