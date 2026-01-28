//! Download cache management

use crate::error::{ColdbrewError, Result};
use crate::storage::paths::Paths;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Cache manager for downloaded bottles
pub struct Cache {
    paths: Paths,
}

impl Cache {
    /// Create a new Cache manager
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    /// Initialize the cache directory
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.paths.downloads_dir())?;
        Ok(())
    }

    /// Get the cache path for a bottle
    pub fn bottle_path(&self, name: &str, version: &str, tag: &str) -> PathBuf {
        self.paths.cache_bottle(name, version, tag)
    }

    /// Check if a bottle is cached
    pub fn is_cached(&self, name: &str, version: &str, tag: &str) -> bool {
        self.bottle_path(name, version, tag).exists()
    }

    /// Get a cached bottle path if it exists
    pub fn get_cached(&self, name: &str, version: &str, tag: &str) -> Option<PathBuf> {
        let path = self.bottle_path(name, version, tag);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Store a bottle in the cache
    pub fn store(&self, name: &str, version: &str, tag: &str, data: &[u8]) -> Result<PathBuf> {
        self.init()?;
        let path = self.bottle_path(name, version, tag);
        fs::write(&path, data)?;
        Ok(path)
    }

    /// Move a downloaded file to the cache
    pub fn move_to_cache(&self, src: &Path, name: &str, version: &str, tag: &str) -> Result<PathBuf> {
        self.init()?;
        let dest = self.bottle_path(name, version, tag);
        fs::rename(src, &dest)?;
        Ok(dest)
    }

    /// Remove a cached bottle
    pub fn remove(&self, name: &str, version: &str, tag: &str) -> Result<()> {
        let path = self.bottle_path(name, version, tag);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// List all cached bottles
    pub fn list(&self) -> Result<Vec<CachedBottle>> {
        let downloads_dir = self.paths.downloads_dir();
        if !downloads_dir.exists() {
            return Ok(Vec::new());
        }

        let mut bottles = Vec::new();
        for entry in fs::read_dir(&downloads_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(bottle) = self.parse_bottle_filename(&path) {
                bottles.push(bottle);
            }
        }

        bottles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(bottles)
    }

    /// Parse a bottle filename into its components
    fn parse_bottle_filename(&self, path: &Path) -> Option<CachedBottle> {
        let filename = path.file_name()?.to_str()?;

        // Format: name-version.tag.bottle.tar.gz
        if !filename.ends_with(".bottle.tar.gz") {
            return None;
        }

        let without_suffix = filename.strip_suffix(".bottle.tar.gz")?;
        let parts: Vec<&str> = without_suffix.rsplitn(2, '.').collect();

        if parts.len() != 2 {
            return None;
        }

        let tag = parts[0];
        let name_version = parts[1];

        // Split name and version at the last hyphen
        let hyphen_idx = name_version.rfind('-')?;
        let name = &name_version[..hyphen_idx];
        let version = &name_version[hyphen_idx + 1..];

        let metadata = path.metadata().ok()?;
        let size = metadata.len();
        let modified = metadata.modified().ok()?;

        Some(CachedBottle {
            name: name.to_string(),
            version: version.to_string(),
            tag: tag.to_string(),
            path: path.to_path_buf(),
            size,
            modified,
        })
    }

    /// Clean up old cached files
    pub fn clean(&self, max_age: Option<Duration>) -> Result<CleanResult> {
        let downloads_dir = self.paths.downloads_dir();
        if !downloads_dir.exists() {
            return Ok(CleanResult::default());
        }

        let now = SystemTime::now();
        let mut removed = 0;
        let mut freed = 0;

        for entry in fs::read_dir(&downloads_dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;

            let should_remove = if let Some(max_age) = max_age {
                if let Ok(modified) = metadata.modified() {
                    now.duration_since(modified).unwrap_or(Duration::ZERO) > max_age
                } else {
                    false
                }
            } else {
                true // Remove all if no max_age specified
            };

            if should_remove {
                let size = metadata.len();
                fs::remove_file(&path)?;
                removed += 1;
                freed += size;
            }
        }

        Ok(CleanResult { removed, freed })
    }

    /// Get total cache size
    pub fn total_size(&self) -> Result<u64> {
        let downloads_dir = self.paths.downloads_dir();
        if !downloads_dir.exists() {
            return Ok(0);
        }

        let mut total = 0;
        for entry in fs::read_dir(&downloads_dir)? {
            let entry = entry?;
            total += entry.metadata()?.len();
        }

        Ok(total)
    }
}

/// Information about a cached bottle
#[derive(Debug)]
pub struct CachedBottle {
    pub name: String,
    pub version: String,
    pub tag: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

/// Result of a cache clean operation
#[derive(Debug, Default)]
pub struct CleanResult {
    pub removed: usize,
    pub freed: u64,
}

impl CleanResult {
    /// Format freed bytes as human-readable
    pub fn freed_human(&self) -> String {
        format_bytes(self.freed)
    }
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_store_and_retrieve() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        let cache = Cache::new(paths);

        let data = b"test bottle data";
        let path = cache.store("jq", "1.7.1", "arm64_sonoma", data).unwrap();

        assert!(path.exists());
        assert!(cache.is_cached("jq", "1.7.1", "arm64_sonoma"));

        let cached_path = cache.get_cached("jq", "1.7.1", "arm64_sonoma").unwrap();
        let content = fs::read(&cached_path).unwrap();
        assert_eq!(content, data);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1_572_864), "1.50 MB");
        assert_eq!(format_bytes(1_610_612_736), "1.50 GB");
    }
}
