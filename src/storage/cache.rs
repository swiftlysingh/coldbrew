//! Download cache management

use crate::error::Result;
use crate::storage::db::BlobCacheEntry;
use crate::storage::paths::Paths;
use crate::storage::Database;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
        fs::create_dir_all(self.paths.cache_blobs_dir())?;
        Ok(())
    }

    /// Get the blob cache path for a bottle
    pub fn blob_path(&self, sha256: &str) -> PathBuf {
        self.paths.cache_blob(sha256)
    }

    /// Get the temp path for an in-flight blob
    pub fn blob_temp_path(&self, sha256: &str) -> PathBuf {
        self.paths.cache_blob_temp(sha256)
    }

    /// Check if a blob is cached
    pub fn is_cached(&self, sha256: &str) -> bool {
        self.blob_path(sha256).exists()
    }

    /// Get a cached blob path if it exists
    pub fn get_cached(&self, sha256: &str) -> Option<PathBuf> {
        let path = self.blob_path(sha256);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Store a blob in the cache
    pub fn store_blob(&self, sha256: &str, data: &[u8]) -> Result<PathBuf> {
        self.init()?;
        let path = self.blob_path(sha256);
        fs::write(&path, data)?;
        Ok(path)
    }

    /// Move a downloaded file to the blob cache
    pub fn move_to_cache(&self, src: &Path, sha256: &str) -> Result<PathBuf> {
        self.init()?;
        let dest = self.blob_path(sha256);
        fs::rename(src, &dest)?;
        Ok(dest)
    }

    pub fn record_blob_metadata(
        &self,
        sha256: &str,
        name: Option<&str>,
        version: Option<&str>,
        tag: Option<&str>,
        size_bytes: u64,
    ) -> Result<()> {
        let db = Database::new(self.paths.clone());
        let conn = db.connect()?;
        db.upsert_blob_cache(&conn, sha256, name, version, tag, size_bytes)?;
        Ok(())
    }

    /// Remove a cached blob
    pub fn remove(&self, sha256: &str) -> Result<()> {
        let path = self.blob_path(sha256);
        if path.exists() {
            fs::remove_file(path)?;
        }

        let db = Database::new(self.paths.clone());
        let conn = db.connect()?;
        db.delete_blob_cache(&conn, sha256)?;
        Ok(())
    }

    /// List all cached bottles
    pub fn list(&self) -> Result<Vec<CachedBottle>> {
        let blob_dir = self.paths.cache_blobs_dir();
        if !blob_dir.exists() {
            return Ok(Vec::new());
        }

        let db = Database::new(self.paths.clone());
        let conn = db.connect()?;
        let entries = db.list_blob_cache(&conn)?;
        let mut bottles = self.materialize_cache_entries(&entries);
        if bottles.is_empty() {
            bottles = self.scan_blob_dir(&blob_dir)?;
        }
        Ok(bottles)
    }

    fn materialize_cache_entries(&self, entries: &[BlobCacheEntry]) -> Vec<CachedBottle> {
        let mut bottles = Vec::new();
        for entry in entries {
            let path = self.blob_path(&entry.sha256);
            if !path.exists() {
                continue;
            }
            let metadata = match path.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let modified = match metadata.modified() {
                Ok(modified) => modified,
                Err(_) => continue,
            };
            bottles.push(CachedBottle {
                sha256: entry.sha256.clone(),
                name: entry.name.clone(),
                version: entry.version.clone(),
                tag: entry.tag.clone(),
                path,
                size: entry.size_bytes,
                modified,
            });
        }

        bottles.sort_by_key(|bottle| bottle.label());
        bottles
    }

    fn scan_blob_dir(&self, blob_dir: &Path) -> Result<Vec<CachedBottle>> {
        let mut bottles = Vec::new();
        for entry in fs::read_dir(blob_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                if let Some(sha) = file_name.strip_suffix(".bottle.tar.gz") {
                    let metadata = entry.metadata()?;
                    bottles.push(CachedBottle {
                        sha256: sha.to_string(),
                        name: None,
                        version: None,
                        tag: None,
                        path,
                        size: metadata.len(),
                        modified: metadata.modified().unwrap_or(UNIX_EPOCH),
                    });
                }
            }
        }
        bottles.sort_by_key(|bottle| bottle.label());
        Ok(bottles)
    }

    /// Clean up old cached files
    pub fn clean(&self, max_age: Option<Duration>) -> Result<CleanResult> {
        let blob_dir = self.paths.cache_blobs_dir();
        if !blob_dir.exists() {
            return Ok(CleanResult::default());
        }

        let now = SystemTime::now();
        let mut removed = 0;
        let mut freed = 0;

        let db = Database::new(self.paths.clone());
        let conn = db.connect()?;
        for entry in fs::read_dir(&blob_dir)? {
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
                let file_name = path.file_name().and_then(|name| name.to_str());
                if let Some(file_name) = file_name {
                    if let Some(sha) = file_name.strip_suffix(".bottle.tar.gz") {
                        db.delete_blob_cache(&conn, sha)?;
                    }
                }
                fs::remove_file(&path)?;
                removed += 1;
                freed += size;
            }
        }

        Ok(CleanResult { removed, freed })
    }

    /// Get total cache size
    pub fn total_size(&self) -> Result<u64> {
        let blob_dir = self.paths.cache_blobs_dir();
        if !blob_dir.exists() {
            return Ok(0);
        }

        let mut total = 0;
        for entry in fs::read_dir(&blob_dir)? {
            let entry = entry?;
            total += entry.metadata()?.len();
        }

        Ok(total)
    }
}

/// Information about a cached bottle
#[derive(Debug)]
pub struct CachedBottle {
    pub sha256: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub tag: Option<String>,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

impl CachedBottle {
    pub fn label(&self) -> String {
        if let (Some(name), Some(version), Some(tag)) =
            (self.name.as_ref(), self.version.as_ref(), self.tag.as_ref())
        {
            format!("{} {} ({})", name, version, tag)
        } else {
            let short = self.sha256.chars().take(12).collect::<String>();
            format!("blob {}", short)
        }
    }
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
        let path = cache.store_blob("abc123", data).unwrap();

        assert!(path.exists());
        assert!(cache.is_cached("abc123"));

        let cached_path = cache.get_cached("abc123").unwrap();
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
