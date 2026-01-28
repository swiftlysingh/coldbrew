//! Lockfile (coldbrew.lock)

use crate::config::ProjectConfig;
use crate::core::version::parse_package_spec;
use crate::error::{ColdbrewError, Result};
use crate::registry::Index;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Lockfile for reproducible installations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// Lockfile version
    pub version: u32,

    /// When the lockfile was generated
    pub generated_at: DateTime<Utc>,

    /// Locked packages
    pub packages: HashMap<String, LockedPackage>,

    /// Checksum of the coldbrew.toml (for sync detection)
    pub config_hash: String,
}

/// A locked package with exact version and checksum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Exact version
    pub version: String,

    /// SHA256 checksum of the bottle
    pub sha256: Option<String>,

    /// Bottle tag used
    pub bottle_tag: Option<String>,

    /// Tap source
    pub tap: String,

    /// Resolved dependencies
    pub dependencies: Vec<String>,

    /// Whether this is a dev dependency
    #[serde(default)]
    pub dev: bool,
}

impl Lockfile {
    /// Generate a lockfile from project config
    pub async fn generate(config: &ProjectConfig, index: &Index) -> Result<Self> {
        let mut packages = HashMap::new();

        // Process regular packages
        for (name, spec) in &config.packages {
            let locked = Self::resolve_package(name, spec.version(), index, false)?;
            packages.insert(name.clone(), locked);
        }

        // Process dev packages
        for (name, spec) in &config.dev_packages {
            let locked = Self::resolve_package(name, spec.version(), index, true)?;
            packages.insert(name.clone(), locked);
        }

        // Calculate config hash
        let config_content = toml::to_string(config)
            .map_err(|e| ColdbrewError::Other(e.to_string()))?;
        let config_hash = Self::hash_string(&config_content);

        Ok(Self {
            version: 1,
            generated_at: Utc::now(),
            packages,
            config_hash,
        })
    }

    fn resolve_package(
        name: &str,
        version_constraint: &str,
        index: &Index,
        dev: bool,
    ) -> Result<LockedPackage> {
        let formula = index
            .get_formula(name)?
            .ok_or_else(|| ColdbrewError::PackageNotFound(name.to_string()))?;

        // For now, just use the stable version
        // TODO: Implement version constraint matching
        let version = formula.versions.stable.clone();

        // Get bottle info
        let (sha256, bottle_tag) = if let Some(ref stable) = formula.bottle.stable {
            // Get the first available bottle for reference
            stable
                .files
                .iter()
                .next()
                .map(|(tag, file)| (Some(file.sha256.clone()), Some(tag.clone())))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        Ok(LockedPackage {
            version,
            sha256,
            bottle_tag,
            tap: formula.tap.clone(),
            dependencies: formula.dependencies.clone(),
            dev,
        })
    }

    fn hash_string(s: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Load a lockfile from disk
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(ColdbrewError::LockfileNotFound);
        }

        let content = fs::read_to_string(path)?;
        let lockfile: Lockfile = toml::from_str(&content)?;
        Ok(lockfile)
    }

    /// Save the lockfile to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ColdbrewError::Other(e.to_string()))?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Check if the lockfile is in sync with the config
    pub fn is_in_sync(&self, config: &ProjectConfig) -> bool {
        let config_content = match toml::to_string(config) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let current_hash = Self::hash_string(&config_content);
        self.config_hash == current_hash
    }

    /// Get package versions as a HashMap
    pub fn package_versions(&self) -> HashMap<String, String> {
        self.packages
            .iter()
            .map(|(name, pkg)| (name.clone(), pkg.version.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_hash() {
        let hash1 = Lockfile::hash_string("test");
        let hash2 = Lockfile::hash_string("test");
        let hash3 = Lockfile::hash_string("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
