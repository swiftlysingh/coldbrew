//! Path utilities for Coldbrew directory structure

use crate::error::{ColdbrewError, Result};
use directories::BaseDirs;
use std::path::{Path, PathBuf};

/// Coldbrew directory structure manager
#[derive(Debug, Clone)]
pub struct Paths {
    /// Root directory (~/.coldbrew)
    root: PathBuf,
}

impl Paths {
    /// Create a new Paths instance with the default root (~/.coldbrew)
    pub fn new() -> Result<Self> {
        let base_dirs = BaseDirs::new().ok_or_else(|| {
            ColdbrewError::Other("Could not determine home directory".to_string())
        })?;

        let root = base_dirs.home_dir().join(".coldbrew");
        Ok(Self { root })
    }

    /// Create a new Paths instance with a custom root (for testing)
    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// Initialize the directory structure
    pub fn init(&self) -> Result<()> {
        let dirs = [
            self.root(),
            &self.bin_dir(),
            &self.cellar_dir(),
            &self.cache_dir(),
            &self.taps_dir(),
            &self.index_dir(),
            &self.logs_dir(),
        ];

        for dir in dirs {
            if !dir.exists() {
                std::fs::create_dir_all(dir)
                    .map_err(|_| ColdbrewError::DirectoryCreationFailed(dir.to_path_buf()))?;
            }
        }

        Ok(())
    }

    /// Root directory (~/.coldbrew)
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Bin directory for shims (~/.coldbrew/bin)
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// Cellar directory for installed packages (~/.coldbrew/cellar)
    pub fn cellar_dir(&self) -> PathBuf {
        self.root.join("cellar")
    }

    /// Cache directory for downloads (~/.coldbrew/cache)
    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// Downloads subdirectory (~/.coldbrew/cache/downloads)
    pub fn downloads_dir(&self) -> PathBuf {
        self.cache_dir().join("downloads")
    }

    /// Taps directory (~/.coldbrew/taps)
    pub fn taps_dir(&self) -> PathBuf {
        self.root.join("taps")
    }

    /// Index directory for formula cache (~/.coldbrew/index)
    pub fn index_dir(&self) -> PathBuf {
        self.root.join("index")
    }

    /// Logs directory (~/.coldbrew/logs)
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Global config file (~/.coldbrew/config.toml)
    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// Formula index file (~/.coldbrew/index/formula.json)
    pub fn formula_index(&self) -> PathBuf {
        self.index_dir().join("formula.json")
    }

    /// Get the cellar path for a specific package version
    /// e.g., ~/.coldbrew/cellar/jq/1.7.1
    pub fn cellar_package(&self, name: &str, version: &str) -> PathBuf {
        self.cellar_dir().join(name).join(version)
    }

    /// Get the tap directory for a specific tap
    /// e.g., ~/.coldbrew/taps/homebrew/core
    pub fn tap_dir(&self, user: &str, repo: &str) -> PathBuf {
        self.taps_dir().join(user).join(repo)
    }

    /// Get the cache path for a downloaded bottle
    /// e.g., ~/.coldbrew/cache/downloads/jq-1.7.1.arm64_sequoia.bottle.tar.gz
    pub fn cache_bottle(&self, name: &str, version: &str, tag: &str) -> PathBuf {
        self.downloads_dir()
            .join(format!("{}-{}.{}.bottle.tar.gz", name, version, tag))
    }

    /// Get the shim path for a binary
    /// e.g., ~/.coldbrew/bin/jq
    pub fn shim(&self, name: &str) -> PathBuf {
        self.bin_dir().join(name)
    }

    /// Get the installed packages metadata file
    /// e.g., ~/.coldbrew/cellar/jq/1.7.1/.coldbrew.json
    pub fn package_metadata(&self, name: &str, version: &str) -> PathBuf {
        self.cellar_package(name, version).join(".coldbrew.json")
    }

    /// Get the defaults file for tracking default versions
    /// e.g., ~/.coldbrew/defaults.json
    pub fn defaults_file(&self) -> PathBuf {
        self.root.join("defaults.json")
    }

    /// Get the pins file for tracking pinned packages
    /// e.g., ~/.coldbrew/pins.json
    pub fn pins_file(&self) -> PathBuf {
        self.root.join("pins.json")
    }

    /// Check if a directory is within the coldbrew root
    pub fn is_coldbrew_path(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
}

impl Default for Paths {
    fn default() -> Self {
        Self::new().expect("Failed to initialize paths")
    }
}

/// Get the project file path (coldbrew.toml) in the current or parent directories
pub fn find_project_file(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let project_file = current.join("coldbrew.toml");
        if project_file.exists() {
            return Some(project_file);
        }

        if !current.pop() {
            return None;
        }
    }
}

/// Get the lockfile path (coldbrew.lock) relative to a project file
pub fn lockfile_path(project_file: &Path) -> PathBuf {
    project_file.with_file_name("coldbrew.lock")
}

/// Find version files in the current or parent directories
/// Supports: .nvmrc, .node-version, .python-version, .ruby-version, .tool-versions
pub fn find_version_file(start_dir: &Path, tool: &str) -> Option<PathBuf> {
    let version_files: Vec<&str> = match tool {
        "node" | "nodejs" => vec![".nvmrc", ".node-version"],
        "python" | "python3" => vec![".python-version"],
        "ruby" => vec![".ruby-version"],
        _ => vec![],
    };

    // Also check .tool-versions (asdf-style)
    let mut all_files = version_files;
    all_files.push(".tool-versions");

    let mut current = start_dir.to_path_buf();

    loop {
        for file in &all_files {
            let version_file = current.join(file);
            if version_file.exists() {
                return Some(version_file);
            }
        }

        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_paths_structure() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());

        assert_eq!(paths.bin_dir(), temp.path().join("bin"));
        assert_eq!(paths.cellar_dir(), temp.path().join("cellar"));
        assert_eq!(paths.cache_dir(), temp.path().join("cache"));
    }

    #[test]
    fn test_cellar_package_path() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());

        let pkg_path = paths.cellar_package("jq", "1.7.1");
        assert_eq!(
            pkg_path,
            temp.path().join("cellar").join("jq").join("1.7.1")
        );
    }

    #[test]
    fn test_init_creates_directories() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());

        paths.init().unwrap();

        assert!(paths.bin_dir().exists());
        assert!(paths.cellar_dir().exists());
        assert!(paths.cache_dir().exists());
    }

    #[test]
    fn test_find_project_file() {
        let temp = TempDir::new().unwrap();
        let project_file = temp.path().join("coldbrew.toml");
        std::fs::write(&project_file, "").unwrap();

        let subdir = temp.path().join("src").join("lib");
        std::fs::create_dir_all(&subdir).unwrap();

        let found = find_project_file(&subdir);
        assert_eq!(found, Some(project_file));
    }
}
