//! Shim generation and management
//!
//! Shims are lightweight wrappers that intercept commands and redirect them
//! to the appropriate package version based on project configuration.

use crate::error::{ColdbrewError, Result};
use crate::storage::paths::Paths;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Manages shims for installed packages
pub struct ShimManager {
    paths: Paths,
}

impl ShimManager {
    /// Create a new ShimManager
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    /// Create shims for all binaries in a package
    pub fn create_shims(
        &self,
        name: &str,
        version: &str,
        binaries: &[String],
    ) -> Result<Vec<PathBuf>> {
        let bin_dir = self.paths.bin_dir();
        fs::create_dir_all(&bin_dir)?;

        let mut created = Vec::new();

        for binary in binaries {
            let shim_path = bin_dir.join(binary);
            self.create_shim(&shim_path, name, version, binary)?;
            created.push(shim_path);
        }

        Ok(created)
    }

    /// Create a single shim
    fn create_shim(
        &self,
        shim_path: &PathBuf,
        package: &str,
        _version: &str,
        binary: &str,
    ) -> Result<()> {
        // The shim is a shell script that calls crew to resolve and exec the binary
        let shim_content = format!(
            r#"#!/bin/sh
# Coldbrew shim for {package}/{binary}
# This shim resolves the correct version and executes the real binary

exec crew exec {package} {binary} "$@"
"#,
            package = package,
            binary = binary,
        );

        fs::write(shim_path, shim_content)?;

        // Make the shim executable
        let mut perms = fs::metadata(shim_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(shim_path, perms)?;

        Ok(())
    }

    /// Remove shims for a package
    pub fn remove_shims(&self, binaries: &[String]) -> Result<()> {
        let bin_dir = self.paths.bin_dir();

        for binary in binaries {
            let shim_path = bin_dir.join(binary);
            if shim_path.exists() {
                // Check if it's a coldbrew shim before removing
                if self.is_coldbrew_shim(&shim_path)? {
                    fs::remove_file(&shim_path)?;
                }
            }
        }

        Ok(())
    }

    /// Check if a file is a coldbrew shim
    fn is_coldbrew_shim(&self, path: &PathBuf) -> Result<bool> {
        let content = fs::read_to_string(path)?;
        Ok(content.contains("# Coldbrew shim"))
    }

    /// List all shims
    pub fn list_shims(&self) -> Result<Vec<ShimInfo>> {
        let bin_dir = self.paths.bin_dir();
        if !bin_dir.exists() {
            return Ok(Vec::new());
        }

        let mut shims = Vec::new();

        for entry in fs::read_dir(&bin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && self.is_coldbrew_shim(&path)? {
                let name = entry.file_name().to_string_lossy().to_string();
                let content = fs::read_to_string(&path)?;

                // Parse the package name from the shim
                if let Some(package) = self.parse_shim_package(&content) {
                    shims.push(ShimInfo {
                        name,
                        package,
                        path,
                    });
                }
            }
        }

        shims.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(shims)
    }

    /// Parse the package name from a shim's content
    fn parse_shim_package(&self, content: &str) -> Option<String> {
        // Look for "# Coldbrew shim for package/binary"
        for line in content.lines() {
            if line.starts_with("# Coldbrew shim for ") {
                let rest = line.strip_prefix("# Coldbrew shim for ")?;
                let package = rest.split('/').next()?;
                return Some(package.to_string());
            }
        }
        None
    }

    /// Check if a shim exists for a binary
    pub fn has_shim(&self, binary: &str) -> bool {
        self.paths.shim(binary).exists()
    }

    /// Get the real binary path for a package version
    pub fn real_binary_path(&self, name: &str, version: &str, binary: &str) -> PathBuf {
        self.paths
            .cellar_package(name, version)
            .join("bin")
            .join(binary)
    }

    /// Resolve which binary to execute based on configuration
    pub fn resolve_binary(
        &self,
        package: &str,
        binary: &str,
        defaults: &HashMap<String, String>,
        project_versions: Option<&HashMap<String, String>>,
    ) -> Result<PathBuf> {
        // Priority: project config > global default > latest installed
        let version = project_versions
            .and_then(|p| p.get(package))
            .or_else(|| defaults.get(package))
            .ok_or_else(|| ColdbrewError::NoDefaultVersion(package.to_string()))?;

        let binary_path = self.real_binary_path(package, version, binary);

        if !binary_path.exists() {
            return Err(ColdbrewError::PackageNotInstalled {
                name: package.to_string(),
                version: version.clone(),
            });
        }

        Ok(binary_path)
    }
}

/// Information about a shim
#[derive(Debug)]
pub struct ShimInfo {
    pub name: String,
    pub package: String,
    pub path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_shim() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        paths.init().unwrap();

        let manager = ShimManager::new(paths.clone());
        let shims = manager
            .create_shims("jq", "1.7.1", &["jq".to_string()])
            .unwrap();

        assert_eq!(shims.len(), 1);
        assert!(shims[0].exists());

        let content = fs::read_to_string(&shims[0]).unwrap();
        assert!(content.contains("# Coldbrew shim"));
        assert!(content.contains("crew exec jq jq"));
    }

    #[test]
    fn test_list_shims() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        paths.init().unwrap();

        let manager = ShimManager::new(paths);
        manager
            .create_shims("jq", "1.7.1", &["jq".to_string()])
            .unwrap();

        let shims = manager.list_shims().unwrap();
        assert_eq!(shims.len(), 1);
        assert_eq!(shims[0].name, "jq");
        assert_eq!(shims[0].package, "jq");
    }
}
