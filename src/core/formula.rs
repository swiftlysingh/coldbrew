//! Formula data structures from Homebrew API

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::bottle::BottleSpec;

/// A Homebrew formula (package definition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    /// Package name (e.g., "jq")
    pub name: String,

    /// Full name including tap (e.g., "homebrew/core/jq")
    pub full_name: String,

    /// Tap name (e.g., "homebrew/core")
    #[serde(default)]
    pub tap: String,

    /// Package description
    pub desc: Option<String>,

    /// Homepage URL
    pub homepage: Option<String>,

    /// License
    pub license: Option<String>,

    /// Version information
    pub versions: Versions,

    /// Bottle (precompiled binary) specification
    #[serde(default)]
    pub bottle: BottleSpec,

    /// Runtime dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Build-time dependencies
    #[serde(default)]
    pub build_dependencies: Vec<String>,

    /// Optional dependencies
    #[serde(default)]
    pub optional_dependencies: Vec<String>,

    /// Test dependencies
    #[serde(default)]
    pub test_dependencies: Vec<String>,

    /// Recommended dependencies
    #[serde(default)]
    pub recommended_dependencies: Vec<String>,

    /// Whether this package is keg-only (not linked to PATH by default)
    #[serde(default)]
    pub keg_only: bool,

    /// Reason for being keg-only
    pub keg_only_reason: Option<KegOnlyReason>,

    /// Whether this package is deprecated
    #[serde(default)]
    pub deprecated: bool,

    /// Deprecation date
    pub deprecation_date: Option<String>,

    /// Deprecation reason
    pub deprecation_reason: Option<String>,

    /// Whether this package is disabled
    #[serde(default)]
    pub disabled: bool,

    /// Disable date
    pub disable_date: Option<String>,

    /// Disable reason
    pub disable_reason: Option<String>,

    /// Post-install caveats
    pub caveats: Option<String>,

    /// URLs for various versions
    #[serde(default)]
    pub urls: HashMap<String, UrlSpec>,

    /// Revision number
    #[serde(default)]
    pub revision: u32,

    /// Version scheme
    #[serde(default)]
    pub version_scheme: u32,

    /// Link overwrite files
    #[serde(default)]
    pub link_overwrite: Vec<String>,

    /// Post-install defined
    #[serde(default)]
    pub post_install_defined: bool,

    /// Service defined
    #[serde(default)]
    pub service: Option<serde_json::Value>,

    /// Analytics install data (30 days)
    pub analytics: Option<Analytics>,

    /// Installation count (30 days)
    #[serde(default)]
    pub analytics_install_on_request_30d: Option<i64>,
}

/// Version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versions {
    /// Stable version
    pub stable: String,

    /// HEAD version (from git)
    pub head: Option<String>,

    /// Bottle available
    #[serde(default)]
    pub bottle: bool,
}

/// URL specification for sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlSpec {
    pub url: String,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub using: Option<String>,
    #[serde(default)]
    pub checksum: Option<String>,
}

/// Reason for being keg-only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KegOnlyReason {
    pub reason: String,
    pub explanation: Option<String>,
}

/// Analytics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analytics {
    #[serde(default)]
    pub install: AnalyticsInstall,
    #[serde(default)]
    pub install_on_request: AnalyticsInstall,
    #[serde(default)]
    pub build_error: AnalyticsBuildError,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsInstall {
    #[serde(rename = "30d", default)]
    pub thirty_days: HashMap<String, i64>,
    #[serde(rename = "90d", default)]
    pub ninety_days: HashMap<String, i64>,
    #[serde(rename = "365d", default)]
    pub one_year: HashMap<String, i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsBuildError {
    #[serde(rename = "30d", default)]
    pub thirty_days: HashMap<String, i64>,
}

impl Formula {
    /// Get the latest stable version string
    pub fn version(&self) -> &str {
        &self.versions.stable
    }

    /// Get the stable version including revision suffix, if any
    pub fn version_with_revision(&self) -> String {
        if self.revision > 0 {
            format!("{}_{}", self.versions.stable, self.revision)
        } else {
            self.versions.stable.clone()
        }
    }

    /// Check if this formula has a bottle for the given platform tag
    pub fn has_bottle(&self, tag: &str) -> bool {
        self.bottle
            .stable
            .as_ref()
            .map(|files| files.files.contains_key(tag))
            .unwrap_or(false)
    }

    /// Get bottle files for a specific tag
    pub fn bottle_for_tag(&self, tag: &str) -> Option<&super::bottle::BottleFile> {
        self.bottle
            .stable
            .as_ref()
            .and_then(|files| files.files.get(tag))
    }

    /// Get all available bottle tags
    pub fn available_bottle_tags(&self) -> Vec<&str> {
        self.bottle
            .stable
            .as_ref()
            .map(|files| files.files.keys().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get all runtime dependencies
    pub fn all_dependencies(&self) -> Vec<&str> {
        self.dependencies.iter().map(|s| s.as_str()).collect()
    }

    /// Check if this is a simple formula (no post-install hooks, no services)
    pub fn is_simple(&self) -> bool {
        !self.post_install_defined && self.service.is_none()
    }

    /// Get formatted display name with version
    pub fn display_name(&self) -> String {
        format!("{} {}", self.name, self.versions.stable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_version() {
        let formula = Formula {
            name: "jq".to_string(),
            full_name: "homebrew/core/jq".to_string(),
            tap: "homebrew/core".to_string(),
            desc: Some("Lightweight JSON processor".to_string()),
            homepage: Some("https://jqlang.github.io/jq/".to_string()),
            license: Some("MIT".to_string()),
            versions: Versions {
                stable: "1.7.1".to_string(),
                head: None,
                bottle: true,
            },
            bottle: BottleSpec::default(),
            dependencies: vec![],
            build_dependencies: vec![],
            optional_dependencies: vec![],
            test_dependencies: vec![],
            recommended_dependencies: vec![],
            keg_only: false,
            keg_only_reason: None,
            deprecated: false,
            deprecation_date: None,
            deprecation_reason: None,
            disabled: false,
            disable_date: None,
            disable_reason: None,
            caveats: None,
            urls: HashMap::new(),
            revision: 0,
            version_scheme: 0,
            link_overwrite: vec![],
            post_install_defined: false,
            service: None,
            analytics: None,
            analytics_install_on_request_30d: None,
        };

        assert_eq!(formula.version(), "1.7.1");
        assert_eq!(formula.display_name(), "jq 1.7.1");
        assert!(formula.is_simple());
    }
}
