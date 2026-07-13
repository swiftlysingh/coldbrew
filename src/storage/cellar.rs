//! Cellar management for installed packages

use crate::core::package::{InstalledPackage, PackageMetadata};
use crate::error::{ColdbrewError, Result};
use crate::storage::paths::Paths;
use std::fs;
use std::path::{Path, PathBuf};

/// Manages the cellar where packages are installed
pub struct Cellar {
    paths: Paths,
}

impl Cellar {
    /// Create a new Cellar manager
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    /// Get all installed packages
    pub fn list_packages(&self) -> Result<Vec<InstalledPackage>> {
        let cellar_dir = self.paths.cellar_dir();
        if !cellar_dir.exists() {
            return Ok(Vec::new());
        }

        let mut packages = Vec::new();

        for entry in fs::read_dir(&cellar_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                for version_entry in fs::read_dir(entry.path())? {
                    let version_entry = version_entry?;
                    if version_entry.file_type()?.is_dir() {
                        let version = version_entry.file_name().to_string_lossy().to_string();
                        if let Ok(pkg) = self.get_package(&name, &version) {
                            packages.push(pkg);
                        }
                    }
                }
            }
        }

        packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
        Ok(packages)
    }

    /// Get a specific installed package
    pub fn get_package(&self, name: &str, version: &str) -> Result<InstalledPackage> {
        let metadata_path = self.paths.package_metadata(name, version);

        if !metadata_path.exists() {
            return Err(ColdbrewError::PackageNotInstalled {
                name: name.to_string(),
                version: version.to_string(),
            });
        }

        let content = fs::read_to_string(&metadata_path)?;
        let metadata: PackageMetadata = serde_json::from_str(&content)?;
        Ok(metadata.package)
    }

    /// Get all versions of a package
    pub fn get_versions(&self, name: &str) -> Result<Vec<String>> {
        let pkg_dir = self.paths.cellar_dir().join(name);
        if !pkg_dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in fs::read_dir(&pkg_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                versions.push(entry.file_name().to_string_lossy().to_string());
            }
        }

        versions.sort();
        Ok(versions)
    }

    /// Check if a package version is installed
    pub fn is_installed(&self, name: &str, version: &str) -> bool {
        self.paths.cellar_package(name, version).exists()
    }

    /// Get the installation path for a package
    pub fn package_path(&self, name: &str, version: &str) -> PathBuf {
        self.paths.cellar_package(name, version)
    }

    /// Get the latest installed version of a package
    pub fn latest_version(&self, name: &str) -> Result<Option<String>> {
        let versions = self.get_versions(name)?;
        Ok(versions.last().cloned())
    }

    /// Point opt/<name> at the latest installed version, or remove it.
    pub fn update_opt_link(&self, name: &str) -> Result<()> {
        let link = self.paths.opt_package(name);
        match fs::symlink_metadata(&link) {
            Ok(_) => fs::remove_file(&link)?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        if let Some(version) = self.latest_version(name)? {
            fs::create_dir_all(self.paths.opt_dir())?;
            create_symlink(
                &Path::new("..").join("cellar").join(name).join(version),
                &link,
            )?;
        }

        Ok(())
    }

    /// Install a package by extracting a bottle
    pub fn install(
        &self,
        name: &str,
        version: &str,
        bottle_path: &std::path::Path,
    ) -> Result<PathBuf> {
        let target_dir = self.paths.cellar_package(name, version);

        // Create parent directories
        if let Some(parent) = target_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        // Extract the bottle
        self.extract_bottle(bottle_path, &target_dir)?;

        Ok(target_dir)
    }

    /// Extract a bottle tarball to the cellar
    fn extract_bottle(
        &self,
        bottle_path: &std::path::Path,
        target_dir: &std::path::Path,
    ) -> Result<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let file = fs::File::open(bottle_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        // Bottles have a top-level directory like "jq/1.7.1/"
        // We need to strip this and extract to target_dir
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            let path_components: Vec<_> = path.components().collect();

            // Skip the first two components (name/version)
            if path_components.len() > 2 {
                let relative_path: PathBuf = path_components[2..].iter().collect();
                let dest = target_dir.join(&relative_path);

                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }

                entry.unpack(&dest)?;
            }
        }

        Ok(())
    }

    /// Uninstall a specific version of a package
    pub fn uninstall(&self, name: &str, version: &str) -> Result<()> {
        let pkg_path = self.paths.cellar_package(name, version);

        if !pkg_path.exists() {
            return Err(ColdbrewError::PackageNotInstalled {
                name: name.to_string(),
                version: version.to_string(),
            });
        }

        fs::remove_dir_all(&pkg_path)?;

        // Remove the package directory if empty
        let pkg_dir = self.paths.cellar_dir().join(name);
        if pkg_dir.exists() && pkg_dir.read_dir()?.next().is_none() {
            fs::remove_dir(&pkg_dir)?;
        }

        self.update_opt_link(name)?;

        Ok(())
    }

    /// Save package metadata
    pub fn save_metadata(&self, metadata: &PackageMetadata) -> Result<()> {
        let path = self
            .paths
            .package_metadata(&metadata.package.name, &metadata.package.version);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(metadata)?;
        fs::write(&path, content)?;

        Ok(())
    }

    /// Get binaries provided by a package
    pub fn get_binaries(&self, name: &str, version: &str) -> Result<Vec<String>> {
        let bin_dir = self.paths.cellar_package(name, version).join("bin");

        if !bin_dir.exists() {
            return Ok(Vec::new());
        }

        let mut binaries = Vec::new();
        for entry in fs::read_dir(&bin_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                binaries.push(entry.file_name().to_string_lossy().to_string());
            }
        }

        binaries.sort();
        Ok(binaries)
    }

    /// Get disk usage for the cellar
    pub fn disk_usage(&self) -> Result<u64> {
        let cellar_dir = self.paths.cellar_dir();
        if !cellar_dir.exists() {
            return Ok(0);
        }

        let mut total = 0;
        for entry in walkdir::WalkDir::new(&cellar_dir) {
            let entry = entry.map_err(|e| ColdbrewError::Other(e.to_string()))?;
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }

        Ok(total)
    }
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<()> {
    Err(ColdbrewError::Other(
        "Symlinks not supported on this platform".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cellar_list_empty() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        let cellar = Cellar::new(paths);

        let packages = cellar.list_packages().unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn test_cellar_is_installed() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        let cellar = Cellar::new(paths.clone());

        // Create a fake package directory
        let pkg_dir = paths.cellar_package("jq", "1.7.1");
        fs::create_dir_all(&pkg_dir).unwrap();

        assert!(cellar.is_installed("jq", "1.7.1"));
        assert!(!cellar.is_installed("jq", "1.7.0"));
    }

    #[cfg(unix)]
    #[test]
    fn test_opt_link_follows_latest_installed_version() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        let cellar = Cellar::new(paths.clone());

        fs::create_dir_all(paths.cellar_package("ffmpeg", "7.1")).unwrap();
        cellar.update_opt_link("ffmpeg").unwrap();
        assert_eq!(
            fs::read_link(paths.opt_package("ffmpeg")).unwrap(),
            Path::new("..").join("cellar/ffmpeg/7.1")
        );

        fs::create_dir_all(paths.cellar_package("ffmpeg", "8.0")).unwrap();
        cellar.update_opt_link("ffmpeg").unwrap();
        cellar.uninstall("ffmpeg", "8.0").unwrap();
        assert_eq!(
            fs::read_link(paths.opt_package("ffmpeg")).unwrap(),
            Path::new("..").join("cellar/ffmpeg/7.1")
        );

        cellar.uninstall("ffmpeg", "7.1").unwrap();
        assert!(fs::symlink_metadata(paths.opt_package("ffmpeg")).is_err());
    }
}
