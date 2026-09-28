//! Reads and writes the app's JSON files in its data directory
//! (`~/Library/Application Support/<identifier>`).

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The value saved at `path`, or the default when the file is missing or unreadable.
pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    let Ok(text) = std::fs::read_to_string(path) else {
        return T::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("{}: {e}; starting from the defaults", path.display());
        T::default()
    })
}

/// Saves `value` at `path`, through a temporary file so a crash never leaves half a file.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn round_trips_and_falls_back_to_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        assert_eq!(load::<Settings>(&path), Settings::default());

        let settings = Settings {
            search_limit: 3,
            ..Settings::default()
        };
        save(&path, &settings).unwrap();
        assert_eq!(load::<Settings>(&path), settings);

        std::fs::write(&path, "not json").unwrap();
        assert_eq!(load::<Settings>(&path), Settings::default());
    }
}
