//! Installed package representation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// An installed package in the cellar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    /// Package name
    pub name: String,

    /// Installed version
    pub version: String,

    /// Tap it came from (e.g., "homebrew/core")
    pub tap: String,

    /// Full path to the installed package
    pub cellar_path: PathBuf,

    /// When this version was installed
    pub installed_at: DateTime<Utc>,

    /// Runtime dependencies (resolved at install time)
    pub runtime_dependencies: Vec<RuntimeDependency>,

    /// Whether this package is linked (shims created)
    #[serde(default)]
    pub linked: bool,

    /// Whether this package is pinned (won't be upgraded)
    #[serde(default)]
    pub pinned: bool,

    /// The bottle tag used for installation
    pub bottle_tag: Option<String>,

    /// SHA256 of the installed bottle
    pub bottle_sha256: Option<String>,

    /// Whether this package is keg-only
    #[serde(default)]
    pub keg_only: bool,

    /// Caveats from the formula
    pub caveats: Option<String>,

    /// List of binaries provided by this package
    #[serde(default)]
    pub binaries: Vec<String>,

    /// Whether this was installed as a dependency
    #[serde(default)]
    pub installed_as_dependency: bool,

    /// Package that requested this as a dependency (if applicable)
    pub installed_for: Option<String>,
}

/// A runtime dependency with its resolved version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDependency {
    /// Dependency name
    pub name: String,

    /// Resolved version at install time
    pub version: String,

    /// Path to the dependency
    pub path: PathBuf,
}

impl InstalledPackage {
    /// Create a new InstalledPackage
    pub fn new(
        name: String,
        version: String,
        tap: String,
        cellar_path: PathBuf,
    ) -> Self {
        Self {
            name,
            version,
            tap,
            cellar_path,
            installed_at: Utc::now(),
            runtime_dependencies: Vec::new(),
            linked: false,
            pinned: false,
            bottle_tag: None,
            bottle_sha256: None,
            keg_only: false,
            caveats: None,
            binaries: Vec::new(),
            installed_as_dependency: false,
            installed_for: None,
        }
    }

    /// Get the bin directory for this package
    pub fn bin_dir(&self) -> PathBuf {
        self.cellar_path.join("bin")
    }

    /// Get the lib directory for this package
    pub fn lib_dir(&self) -> PathBuf {
        self.cellar_path.join("lib")
    }

    /// Get the include directory for this package
    pub fn include_dir(&self) -> PathBuf {
        self.cellar_path.join("include")
    }

    /// Get the share directory for this package
    pub fn share_dir(&self) -> PathBuf {
        self.cellar_path.join("share")
    }

    /// Check if this package has binaries
    pub fn has_binaries(&self) -> bool {
        !self.binaries.is_empty() || self.bin_dir().exists()
    }

    /// Get the display string for this package
    pub fn display(&self) -> String {
        let mut s = format!("{} {}", self.name, self.version);
        if self.pinned {
            s.push_str(" (pinned)");
        }
        if self.keg_only {
            s.push_str(" (keg-only)");
        }
        s
    }

    /// Get the package identifier (name@version)
    pub fn identifier(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }
}

/// Metadata stored alongside each installed package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package info
    pub package: InstalledPackage,

    /// Original formula (for reference)
    pub formula_json: Option<serde_json::Value>,

    /// Install receipt
    pub receipt: InstallReceipt,
}

/// Installation receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallReceipt {
    /// Coldbrew version used for installation
    pub installed_by: String,

    /// Installation timestamp
    pub installed_at: DateTime<Utc>,

    /// Source (bottle URL or "source")
    pub source: String,

    /// Checksum verification result
    pub checksum_verified: bool,
}

impl PackageMetadata {
    /// Create metadata for a new installation
    pub fn new(package: InstalledPackage, source: String) -> Self {
        Self {
            package,
            formula_json: None,
            receipt: InstallReceipt {
                installed_by: format!("coldbrew {}", env!("CARGO_PKG_VERSION")),
                installed_at: Utc::now(),
                source,
                checksum_verified: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_installed_package_display() {
        let pkg = InstalledPackage::new(
            "jq".to_string(),
            "1.7.1".to_string(),
            "homebrew/core".to_string(),
            PathBuf::from("/home/user/.coldbrew/cellar/jq/1.7.1"),
        );

        assert_eq!(pkg.display(), "jq 1.7.1");
        assert_eq!(pkg.identifier(), "jq@1.7.1");
    }

    #[test]
    fn test_installed_package_pinned() {
        let mut pkg = InstalledPackage::new(
            "node".to_string(),
            "22.0.0".to_string(),
            "homebrew/core".to_string(),
            PathBuf::from("/home/user/.coldbrew/cellar/node/22.0.0"),
        );
        pkg.pinned = true;

        assert_eq!(pkg.display(), "node 22.0.0 (pinned)");
    }
}
