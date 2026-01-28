//! Project configuration (coldbrew.toml)

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Project-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name (optional)
    pub name: Option<String>,

    /// Package dependencies
    #[serde(default)]
    pub packages: HashMap<String, PackageSpec>,

    /// Development-only packages
    #[serde(default)]
    pub dev_packages: HashMap<String, PackageSpec>,
}

/// Package specification in coldbrew.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageSpec {
    /// Just a version string
    Version(String),

    /// Full specification
    Full(PackageSpecFull),
}

/// Full package specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpecFull {
    /// Version constraint
    pub version: String,

    /// Custom tap (optional)
    pub tap: Option<String>,

    /// Skip linking (for keg-only usage)
    #[serde(default)]
    pub skip_link: bool,
}

impl PackageSpec {
    /// Get the version string
    pub fn version(&self) -> &str {
        match self {
            PackageSpec::Version(v) => v,
            PackageSpec::Full(f) => &f.version,
        }
    }

    /// Get the tap (if specified)
    pub fn tap(&self) -> Option<&str> {
        match self {
            PackageSpec::Version(_) => None,
            PackageSpec::Full(f) => f.tap.as_deref(),
        }
    }

    /// Check if linking should be skipped
    pub fn skip_link(&self) -> bool {
        match self {
            PackageSpec::Version(_) => false,
            PackageSpec::Full(f) => f.skip_link,
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: None,
            packages: HashMap::new(),
            dev_packages: HashMap::new(),
        }
    }
}

impl ProjectConfig {
    /// Load a project configuration from a file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save the project configuration to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::ColdbrewError::Other(e.to_string()))?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get all package names and versions
    pub fn all_packages(&self) -> HashMap<String, String> {
        let mut packages = HashMap::new();

        for (name, spec) in &self.packages {
            packages.insert(name.clone(), spec.version().to_string());
        }

        for (name, spec) in &self.dev_packages {
            packages.insert(name.clone(), spec.version().to_string());
        }

        packages
    }

    /// Add a package
    pub fn add_package(&mut self, name: &str, version: &str, dev: bool) {
        let spec = PackageSpec::Version(version.to_string());
        if dev {
            self.dev_packages.insert(name.to_string(), spec);
        } else {
            self.packages.insert(name.to_string(), spec);
        }
    }

    /// Remove a package
    pub fn remove_package(&mut self, name: &str) {
        self.packages.remove(name);
        self.dev_packages.remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_project_config_roundtrip() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("coldbrew.toml");

        let mut config = ProjectConfig::default();
        config.name = Some("test-project".to_string());
        config.add_package("jq", "1.7", false);
        config.add_package("node", "22", true);

        config.save(&config_path).unwrap();

        let loaded = ProjectConfig::load(&config_path).unwrap();
        assert_eq!(loaded.name, Some("test-project".to_string()));
        assert!(loaded.packages.contains_key("jq"));
        assert!(loaded.dev_packages.contains_key("node"));
    }

    #[test]
    fn test_package_spec_version() {
        let spec = PackageSpec::Version("1.7.1".to_string());
        assert_eq!(spec.version(), "1.7.1");
        assert_eq!(spec.tap(), None);
        assert!(!spec.skip_link());
    }

    #[test]
    fn test_package_spec_full() {
        let spec = PackageSpec::Full(PackageSpecFull {
            version: "1.7.1".to_string(),
            tap: Some("user/repo".to_string()),
            skip_link: true,
        });
        assert_eq!(spec.version(), "1.7.1");
        assert_eq!(spec.tap(), Some("user/repo"));
        assert!(spec.skip_link());
    }
}
