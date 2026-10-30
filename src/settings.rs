use std::{env, fs, io, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::{APP_NAME, logic::Repeat};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub roots: Vec<PathBuf>,
    pub last_dir: Option<PathBuf>,
    pub last_file: Option<PathBuf>,
    pub last_pos: f64,
    pub volume: f32,
    pub repeat: Repeat,
    pub shuffle: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            last_dir: None,
            last_file: None,
            last_pos: 0.0,
            volume: 1.0,
            repeat: Repeat::Off,
            shuffle: false,
        }
    }
}

// ponytail: no portable mode. Config always in the user profile.
pub fn path() -> Option<PathBuf> {
    let base = env::var_os("APPDATA")
        .or_else(|| env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join(APP_NAME).join("settings.toml"))
}

/// Missing or broken file gives defaults.
pub fn load(path: &Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = toml::to_string_pretty(settings).map_err(io::Error::other)?;
    fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s = Settings {
            roots: vec![PathBuf::from("/music/a")],
            last_file: Some(PathBuf::from("/music/a/x.mp3")),
            last_pos: 12.5,
            repeat: Repeat::All,
            shuffle: true,
            ..Settings::default()
        };
        let text = toml::to_string_pretty(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
        assert_eq!(load(Path::new("/no/such/file.toml")), Settings::default());
    }
}
