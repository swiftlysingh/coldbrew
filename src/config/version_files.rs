//! Version file detection (.nvmrc, .python-version, etc.)

use crate::error::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Detector for version files
pub struct VersionFileDetector {
    /// Starting directory for search
    start_dir: PathBuf,
}

/// Detected version from a version file
#[derive(Debug, Clone)]
pub struct DetectedVersion {
    /// Package name (normalized)
    pub package: String,
    /// Version string
    pub version: String,
    /// Path to the version file
    pub file_path: PathBuf,
    /// Type of version file
    pub file_type: VersionFileType,
}

/// Type of version file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionFileType {
    Nvmrc,
    NodeVersion,
    PythonVersion,
    RubyVersion,
    ToolVersions,
}

impl VersionFileDetector {
    /// Create a new detector starting from the given directory
    pub fn new(start_dir: PathBuf) -> Self {
        Self { start_dir }
    }

    /// Detect all version files in the directory hierarchy
    pub fn detect_all(&self) -> Result<Vec<DetectedVersion>> {
        let mut versions = Vec::new();
        let mut current = self.start_dir.clone();

        loop {
            // Check for .nvmrc
            let nvmrc = current.join(".nvmrc");
            if nvmrc.exists() {
                if let Some(v) = self.parse_simple_version_file(&nvmrc, "node", VersionFileType::Nvmrc)? {
                    versions.push(v);
                }
            }

            // Check for .node-version
            let node_version = current.join(".node-version");
            if node_version.exists() {
                if let Some(v) = self.parse_simple_version_file(&node_version, "node", VersionFileType::NodeVersion)? {
                    versions.push(v);
                }
            }

            // Check for .python-version
            let python_version = current.join(".python-version");
            if python_version.exists() {
                if let Some(v) = self.parse_simple_version_file(&python_version, "python", VersionFileType::PythonVersion)? {
                    versions.push(v);
                }
            }

            // Check for .ruby-version
            let ruby_version = current.join(".ruby-version");
            if ruby_version.exists() {
                if let Some(v) = self.parse_simple_version_file(&ruby_version, "ruby", VersionFileType::RubyVersion)? {
                    versions.push(v);
                }
            }

            // Check for .tool-versions (asdf-style)
            let tool_versions = current.join(".tool-versions");
            if tool_versions.exists() {
                versions.extend(self.parse_tool_versions(&tool_versions)?);
            }

            // Move up to parent directory
            if !current.pop() {
                break;
            }
        }

        Ok(versions)
    }

    /// Detect version for a specific package
    pub fn detect_for_package(&self, package: &str) -> Result<Option<DetectedVersion>> {
        let all = self.detect_all()?;
        Ok(all.into_iter().find(|v| v.package == package))
    }

    /// Parse a simple version file (one version per file)
    fn parse_simple_version_file(
        &self,
        path: &Path,
        package: &str,
        file_type: VersionFileType,
    ) -> Result<Option<DetectedVersion>> {
        let content = fs::read_to_string(path)?;
        let version = content.trim();

        if version.is_empty() {
            return Ok(None);
        }

        // Clean up version string (remove leading 'v' if present)
        let version = version.strip_prefix('v').unwrap_or(version);

        Ok(Some(DetectedVersion {
            package: package.to_string(),
            version: version.to_string(),
            file_path: path.to_path_buf(),
            file_type,
        }))
    }

    /// Parse .tool-versions file (asdf format)
    fn parse_tool_versions(&self, path: &Path) -> Result<Vec<DetectedVersion>> {
        let content = fs::read_to_string(path)?;
        let mut versions = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Format: "tool version"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let tool = parts[0];
                let version = parts[1];

                // Map asdf tool names to Homebrew package names
                let package = match tool {
                    "nodejs" | "node" => "node",
                    "python" => "python",
                    "ruby" => "ruby",
                    "golang" | "go" => "go",
                    "rust" => "rust",
                    other => other,
                };

                versions.push(DetectedVersion {
                    package: package.to_string(),
                    version: version.to_string(),
                    file_path: path.to_path_buf(),
                    file_type: VersionFileType::ToolVersions,
                });
            }
        }

        Ok(versions)
    }
}

/// Get version files as a HashMap for quick lookup
pub fn get_version_map(start_dir: &Path) -> Result<HashMap<String, String>> {
    let detector = VersionFileDetector::new(start_dir.to_path_buf());
    let versions = detector.detect_all()?;

    let mut map = HashMap::new();
    for v in versions {
        // First match wins (closer to start_dir)
        map.entry(v.package).or_insert(v.version);
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_nvmrc() {
        let temp = TempDir::new().unwrap();
        let nvmrc = temp.path().join(".nvmrc");
        fs::write(&nvmrc, "18.17.0\n").unwrap();

        let detector = VersionFileDetector::new(temp.path().to_path_buf());
        let versions = detector.detect_all().unwrap();

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].package, "node");
        assert_eq!(versions[0].version, "18.17.0");
        assert_eq!(versions[0].file_type, VersionFileType::Nvmrc);
    }

    #[test]
    fn test_detect_tool_versions() {
        let temp = TempDir::new().unwrap();
        let tool_versions = temp.path().join(".tool-versions");
        fs::write(
            &tool_versions,
            "nodejs 18.17.0\npython 3.11.0\n# comment\nruby 3.2.0\n",
        )
        .unwrap();

        let detector = VersionFileDetector::new(temp.path().to_path_buf());
        let versions = detector.detect_all().unwrap();

        assert_eq!(versions.len(), 3);
        assert!(versions.iter().any(|v| v.package == "node" && v.version == "18.17.0"));
        assert!(versions.iter().any(|v| v.package == "python" && v.version == "3.11.0"));
        assert!(versions.iter().any(|v| v.package == "ruby" && v.version == "3.2.0"));
    }

    #[test]
    fn test_strip_v_prefix() {
        let temp = TempDir::new().unwrap();
        let nvmrc = temp.path().join(".nvmrc");
        fs::write(&nvmrc, "v18.17.0").unwrap();

        let detector = VersionFileDetector::new(temp.path().to_path_buf());
        let versions = detector.detect_all().unwrap();

        assert_eq!(versions[0].version, "18.17.0");
    }
}
