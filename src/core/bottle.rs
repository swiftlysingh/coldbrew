//! Bottle (precompiled binary) data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bottle specification from formula JSON
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BottleSpec {
    /// Stable bottle files
    pub stable: Option<BottleFiles>,
}

/// Collection of bottle files for different platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleFiles {
    /// Rebuild number
    #[serde(default)]
    pub rebuild: u32,

    /// Root URL for downloads
    pub root_url: String,

    /// Platform-specific bottle files
    pub files: HashMap<String, BottleFile>,
}

/// Individual bottle file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleFile {
    /// Cellar type (relocatable or fixed path)
    pub cellar: CellarType,

    /// Download URL
    pub url: String,

    /// SHA256 checksum
    pub sha256: String,
}

/// Cellar type indicates how the package was compiled
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum CellarType {
    /// Standard relocatable cellar
    Relocatable(String),
    /// Fixed path (not relocatable)
    Fixed(String),
}

impl CellarType {
    /// Check if this bottle is relocatable
    pub fn is_relocatable(&self) -> bool {
        match self {
            CellarType::Relocatable(s) => s == ":any" || s == ":any_skip_relocation",
            CellarType::Fixed(_) => false,
        }
    }

    /// Get the cellar path string
    pub fn path(&self) -> &str {
        match self {
            CellarType::Relocatable(s) => s,
            CellarType::Fixed(s) => s,
        }
    }
}

impl Default for CellarType {
    fn default() -> Self {
        CellarType::Relocatable(":any_skip_relocation".to_string())
    }
}

impl BottleFile {
    /// Get the GHCR-compatible URL for downloading this bottle
    pub fn ghcr_url(&self, name: &str, version: &str, tag: &str) -> String {
        // The URL in the JSON is already the direct GHCR blob URL
        // But we need to construct it properly for the token auth flow
        format!(
            "https://ghcr.io/v2/homebrew/core/{}/blobs/sha256:{}",
            name,
            // Extract the sha256 from the URL or use the provided one
            self.sha256
        )
    }
}

impl BottleFiles {
    /// Get the best bottle file for the given platform tags
    pub fn best_for_platform(&self, tags: &[String]) -> Option<(&str, &BottleFile)> {
        for tag in tags {
            if let Some(file) = self.files.get(tag) {
                return Some((tag.as_str(), file));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cellar_type_relocatable() {
        let cellar = CellarType::Relocatable(":any_skip_relocation".to_string());
        assert!(cellar.is_relocatable());

        let cellar = CellarType::Relocatable(":any".to_string());
        assert!(cellar.is_relocatable());
    }

    #[test]
    fn test_cellar_type_fixed() {
        let cellar = CellarType::Fixed("/opt/homebrew/Cellar".to_string());
        assert!(!cellar.is_relocatable());
    }

    #[test]
    fn test_bottle_files_best_for_platform() {
        let mut files = HashMap::new();
        files.insert(
            "arm64_sonoma".to_string(),
            BottleFile {
                cellar: CellarType::Relocatable(":any".to_string()),
                url: "https://example.com/bottle.tar.gz".to_string(),
                sha256: "abc123".to_string(),
            },
        );

        let bottle_files = BottleFiles {
            rebuild: 0,
            root_url: "https://ghcr.io".to_string(),
            files,
        };

        let tags = vec![
            "arm64_sequoia".to_string(),
            "arm64_sonoma".to_string(),
            "arm64_ventura".to_string(),
        ];

        let result = bottle_files.best_for_platform(&tags);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "arm64_sonoma");
    }
}
