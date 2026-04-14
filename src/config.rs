use std::path::{Path, PathBuf};

use log::debug;
use serde::Deserialize;

/// Decay configuration loaded from config files.
///
/// Priority (low → high): XDG global → project .decayrc
/// Merge strategy: lists append, scalars override.
#[derive(Debug, Clone, Default)]
pub struct DecayConfig {
    pub exclude_dirs: Vec<String>,
    pub exclude_extensions: Vec<String>,
    /// None = auto-detect, Some = override
    pub languages: Option<Vec<String>>,
}

/// Raw TOML structure.
#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    filter: RawFilter,
}

#[derive(Deserialize, Default)]
struct RawFilter {
    #[serde(default)]
    exclude_dirs: Vec<String>,
    #[serde(default)]
    exclude_extensions: Vec<String>,
    #[serde(default)]
    languages: Option<Vec<String>>,
}

impl DecayConfig {
    /// Load config: XDG global → project .decayrc (merge).
    pub fn load(project_path: &Path) -> Self {
        let global = Self::load_file(&xdg_config_path());
        let local = Self::load_file(&project_path.join(".decayrc"));
        global.merge(&local)
    }

    fn load_file(path: &PathBuf) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        debug!("config: loaded {}", path.display());

        let raw: RawConfig = match toml::from_str(&content) {
            Ok(r) => r,
            Err(e) => {
                debug!("config: parse error in {}: {e}", path.display());
                return Self::default();
            }
        };

        Self {
            exclude_dirs: raw.filter.exclude_dirs,
            exclude_extensions: raw.filter.exclude_extensions,
            languages: raw.filter.languages,
        }
    }

    /// Merge: lists append, languages override (local wins).
    fn merge(&self, other: &Self) -> Self {
        let mut exclude_dirs = self.exclude_dirs.clone();
        exclude_dirs.extend(other.exclude_dirs.iter().cloned());

        let mut exclude_extensions = self.exclude_extensions.clone();
        exclude_extensions.extend(other.exclude_extensions.iter().cloned());

        // Languages: local overrides global
        let languages = if other.languages.is_some() {
            other.languages.clone()
        } else {
            self.languages.clone()
        };

        Self {
            exclude_dirs,
            exclude_extensions,
            languages,
        }
    }
}

/// XDG config path: ~/.config/decay/config.toml
fn xdg_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("decay")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_missing_file() {
        let config = DecayConfig::load_file(&PathBuf::from("/nonexistent/.decayrc"));
        assert!(config.exclude_dirs.is_empty());
        assert!(config.languages.is_none());
    }

    #[test]
    fn test_load_valid_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".decayrc"),
            r#"
[filter]
exclude_dirs = ["DoKit", "FMDB"]
exclude_extensions = ["pbxproj"]
languages = ["swift", "objc"]
"#,
        )
        .unwrap();

        let config = DecayConfig::load(dir.path());
        assert_eq!(config.exclude_dirs, vec!["DoKit", "FMDB"]);
        assert_eq!(config.exclude_extensions, vec!["pbxproj"]);
        assert_eq!(config.languages, Some(vec!["swift".to_string(), "objc".to_string()]));
    }

    #[test]
    fn test_merge_appends_lists() {
        let global = DecayConfig {
            exclude_dirs: vec!["vendor".to_string()],
            exclude_extensions: vec![],
            languages: None,
        };
        let local = DecayConfig {
            exclude_dirs: vec!["DoKit".to_string()],
            exclude_extensions: vec!["pbxproj".to_string()],
            languages: Some(vec!["swift".to_string()]),
        };
        let merged = global.merge(&local);
        assert_eq!(merged.exclude_dirs, vec!["vendor", "DoKit"]);
        assert_eq!(merged.exclude_extensions, vec!["pbxproj"]);
        assert_eq!(merged.languages, Some(vec!["swift".to_string()]));
    }

    #[test]
    fn test_merge_global_languages_used_when_local_none() {
        let global = DecayConfig {
            exclude_dirs: vec![],
            exclude_extensions: vec![],
            languages: Some(vec!["rust".to_string()]),
        };
        let local = DecayConfig::default();
        let merged = global.merge(&local);
        assert_eq!(merged.languages, Some(vec!["rust".to_string()]));
    }

    #[test]
    fn test_load_invalid_toml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".decayrc"), "not valid toml {{{}}}").unwrap();
        let config = DecayConfig::load(dir.path());
        assert!(config.exclude_dirs.is_empty()); // graceful fallback
    }
}
