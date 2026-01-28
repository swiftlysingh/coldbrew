//! Global configuration (~/.coldbrew/config.toml)

use crate::error::Result;
use crate::storage::Paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Global configuration for Coldbrew
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Default versions for packages
    #[serde(default)]
    pub defaults: HashMap<String, String>,

    /// Pinned packages (won't be upgraded)
    #[serde(default)]
    pub pins: HashMap<String, String>,

    /// General settings
    #[serde(default)]
    pub settings: Settings,
}

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Auto-update index on install (default: true)
    #[serde(default = "default_true")]
    pub auto_update: bool,

    /// Number of parallel downloads
    #[serde(default = "default_parallel")]
    pub parallel_downloads: usize,

    /// Keep old versions (default: 2)
    #[serde(default = "default_keep_versions")]
    pub keep_versions: usize,

    /// Enable analytics (default: false)
    #[serde(default)]
    pub analytics: bool,
}

fn default_true() -> bool {
    true
}

fn default_parallel() -> usize {
    4
}

fn default_keep_versions() -> usize {
    2
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_update: true,
            parallel_downloads: 4,
            keep_versions: 2,
            analytics: false,
        }
    }
}

impl GlobalConfig {
    /// Load the global configuration
    pub fn load(paths: &Paths) -> Result<Self> {
        let config_path = paths.config_file();

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: GlobalConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save the global configuration
    pub fn save(&self, paths: &Paths) -> Result<()> {
        let config_path = paths.config_file();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::ColdbrewError::Other(e.to_string()))?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    /// Get the default version for a package
    pub fn get_default(&self, name: &str) -> Option<String> {
        self.defaults.get(name).cloned()
    }

    /// Set the default version for a package
    pub fn set_default(&mut self, name: &str, version: &str) {
        self.defaults.insert(name.to_string(), version.to_string());
    }

    /// Remove the default version for a package
    pub fn remove_default(&mut self, name: &str) {
        self.defaults.remove(name);
    }

    /// Check if a package is pinned
    pub fn is_pinned(&self, name: &str) -> bool {
        self.pins.contains_key(name)
    }

    /// Get the pinned version for a package
    pub fn get_pin(&self, name: &str) -> Option<String> {
        self.pins.get(name).cloned()
    }

    /// Pin a package at a specific version
    pub fn add_pin(&mut self, name: &str, version: &str) {
        self.pins.insert(name.to_string(), version.to_string());
    }

    /// Unpin a package
    pub fn remove_pin(&mut self, name: &str) {
        self.pins.remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert!(config.defaults.is_empty());
        assert!(config.pins.is_empty());
        assert!(config.settings.auto_update);
    }

    #[test]
    fn test_save_and_load() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());

        let mut config = GlobalConfig::default();
        config.set_default("node", "22.0.0");
        config.add_pin("jq", "1.7.1");

        config.save(&paths).unwrap();

        let loaded = GlobalConfig::load(&paths).unwrap();
        assert_eq!(loaded.get_default("node"), Some("22.0.0".to_string()));
        assert!(loaded.is_pinned("jq"));
    }
}
