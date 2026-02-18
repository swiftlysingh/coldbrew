//! Cleanup planning for disk usage

use crate::config::GlobalConfig;
use crate::core::InstalledPackage;
use crate::error::Result;
use crate::registry::TapManager;
use crate::storage::{Cache, Cellar, Database, Paths, ShimManager, Store};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CleanupKind {
    OldVersions,
    CacheDownloads,
    IndexCache,
    BrokenShims,
    OrphanedDependencies,
    UnusedTaps,
    OrphanedStore,
}

#[derive(Debug, Clone)]
pub struct CleanupItem {
    pub label: String,
    pub path: PathBuf,
    pub size: u64,
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CleanupCategory {
    pub kind: CleanupKind,
    pub title: &'static str,
    pub items: Vec<CleanupItem>,
}

impl CleanupCategory {
    pub fn total_size(&self) -> u64 {
        self.items.iter().map(|item| item.size).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct CleanupResult {
    pub removed: usize,
    pub freed: u64,
}

pub fn collect_categories(paths: &Paths) -> Result<Vec<CleanupCategory>> {
    let cellar = Cellar::new(paths.clone());
    let cache = Cache::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());
    let tap_manager = TapManager::new(paths.clone());
    let db = Database::new(paths.clone());
    let store = Store::new(paths.clone());

    let installed = cellar.list_packages()?;

    Ok(vec![
        collect_old_versions(&installed)?,
        collect_cache_downloads(&cache)?,
        collect_index_cache(paths)?,
        collect_broken_shims(&installed, &shim_manager)?,
        collect_orphaned_dependencies(&installed)?,
        collect_unused_taps(&installed, &tap_manager)?,
        collect_orphaned_store(&db, &store)?,
    ])
}

pub fn apply_cleanup(
    paths: &Paths,
    categories: &[CleanupCategory],
    selected: &HashSet<CleanupKind>,
    dry_run: bool,
) -> Result<CleanupResult> {
    let cellar = Cellar::new(paths.clone());
    let cache = Cache::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());
    let mut config = GlobalConfig::load(paths)?;
    let mut config_dirty = false;

    let mut result = CleanupResult::default();

    for category in categories {
        if !selected.contains(&category.kind) || category.items.is_empty() {
            continue;
        }

        match category.kind {
            CleanupKind::OldVersions => {
                for item in &category.items {
                    if !dry_run {
                        if let (Some(name), Some(version)) =
                            (item.name.as_deref(), item.version.as_deref())
                        {
                            cellar.uninstall(name, version)?;
                        }
                    }
                    result.removed += 1;
                    result.freed += item.size;
                }
            }
            CleanupKind::CacheDownloads | CleanupKind::IndexCache | CleanupKind::BrokenShims => {
                for item in &category.items {
                    if !dry_run && item.path.exists() {
                        if category.kind == CleanupKind::CacheDownloads {
                            if let Some(sha) = extract_blob_sha(&item.path) {
                                cache.remove(&sha)?;
                            } else {
                                std::fs::remove_file(&item.path)?;
                            }
                        } else {
                            std::fs::remove_file(&item.path)?;
                        }
                    }
                    result.removed += 1;
                    result.freed += item.size;
                }

                if !dry_run && category.kind == CleanupKind::IndexCache {
                    let index_dir = paths.index_dir();
                    if index_dir.exists() && index_dir.read_dir()?.next().is_none() {
                        std::fs::remove_dir(&index_dir)?;
                    }
                }
            }
            CleanupKind::OrphanedDependencies => {
                let mut by_name: HashMap<String, Vec<&CleanupItem>> = HashMap::new();
                for item in &category.items {
                    if let Some(name) = item.name.clone() {
                        by_name.entry(name).or_default().push(item);
                    }
                }

                for (name, items) in by_name {
                    let versions_to_remove: HashSet<String> = items
                        .iter()
                        .filter_map(|item| item.version.clone())
                        .collect();
                    let versions = cellar.get_versions(&name)?;
                    let remaining: Vec<_> = versions
                        .iter()
                        .filter(|v| !versions_to_remove.contains(*v))
                        .cloned()
                        .collect();

                    let remove_shims = remaining.is_empty();
                    let mut binaries = HashSet::new();

                    if remove_shims {
                        for item in &items {
                            if let Some(version) = &item.version {
                                for binary in cellar.get_binaries(&name, version)? {
                                    binaries.insert(binary);
                                }
                            }
                        }
                    }

                    for item in items {
                        if !dry_run {
                            if let Some(version) = &item.version {
                                cellar.uninstall(&name, version)?;
                            }
                        }
                        result.removed += 1;
                        result.freed += item.size;
                    }

                    if remove_shims && !dry_run {
                        let binaries: Vec<String> = binaries.into_iter().collect();
                        shim_manager.remove_shims(&binaries)?;
                        config.remove_default(&name);
                        config.remove_pin(&name);
                        config_dirty = true;
                    }
                }
            }
            CleanupKind::UnusedTaps => {
                let mut tap_manager = TapManager::new(paths.clone());

                for item in &category.items {
                    if !dry_run {
                        if let Some(name) = item.name.as_deref() {
                            tap_manager.remove(name)?;
                        }
                    }
                    result.removed += 1;
                    result.freed += item.size;
                }
            }
            CleanupKind::OrphanedStore => {
                let db = Database::new(paths.clone());
                let store = Store::new(paths.clone());
                let conn = db.connect()?;

                for item in &category.items {
                    if !dry_run {
                        // item.name contains the full sha256
                        if let Some(sha256) = item.name.as_deref() {
                            // Remove from disk first
                            store.remove_entry(sha256)?;
                            // Then remove from database
                            db.delete_store_entry(&conn, sha256)?;
                        }
                    }
                    result.removed += 1;
                    result.freed += item.size;
                }
            }
        }
    }

    if config_dirty && !dry_run {
        config.save(paths)?;
    }

    Ok(result)
}

fn extract_blob_sha(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    file_name
        .strip_suffix(".bottle.tar.gz")
        .map(|sha| sha.to_string())
}

fn collect_old_versions(installed: &[InstalledPackage]) -> Result<CleanupCategory> {
    let mut by_name: HashMap<String, Vec<&InstalledPackage>> = HashMap::new();
    for pkg in installed {
        by_name.entry(pkg.name.clone()).or_default().push(pkg);
    }

    let mut items = Vec::new();

    for (name, mut versions) in by_name {
        versions.sort_by(|a, b| a.version.cmp(&b.version));
        if versions.len() > 2 {
            for pkg in &versions[..versions.len() - 2] {
                let size = dir_size(&pkg.cellar_path);
                items.push(CleanupItem {
                    label: format!("{} {}", name, pkg.version),
                    path: pkg.cellar_path.clone(),
                    size,
                    name: Some(name.clone()),
                    version: Some(pkg.version.clone()),
                });
            }
        }
    }

    Ok(CleanupCategory {
        kind: CleanupKind::OldVersions,
        title: "Old versions",
        items,
    })
}

fn collect_cache_downloads(cache: &Cache) -> Result<CleanupCategory> {
    let mut items = Vec::new();
    for bottle in cache.list()? {
        items.push(CleanupItem {
            label: bottle.label(),
            path: bottle.path,
            size: bottle.size,
            name: None,
            version: None,
        });
    }

    Ok(CleanupCategory {
        kind: CleanupKind::CacheDownloads,
        title: "Cache downloads",
        items,
    })
}

fn collect_index_cache(paths: &Paths) -> Result<CleanupCategory> {
    let mut items = Vec::new();
    let index_dir = paths.index_dir();
    if index_dir.exists() {
        for entry in walkdir::WalkDir::new(&index_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path().to_path_buf();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let label = path
                    .strip_prefix(&index_dir)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| path.to_string_lossy().to_string());
                items.push(CleanupItem {
                    label,
                    path,
                    size,
                    name: None,
                    version: None,
                });
            }
        }
    }

    Ok(CleanupCategory {
        kind: CleanupKind::IndexCache,
        title: "Index cache",
        items,
    })
}

fn collect_broken_shims(
    installed: &[InstalledPackage],
    shim_manager: &ShimManager,
) -> Result<CleanupCategory> {
    let installed_names: HashSet<String> = installed.iter().map(|pkg| pkg.name.clone()).collect();
    let mut items = Vec::new();
    for shim in shim_manager.list_shims()? {
        if !installed_names.contains(&shim.package) {
            let size = std::fs::metadata(&shim.path).map(|m| m.len()).unwrap_or(0);
            items.push(CleanupItem {
                label: format!("{} (missing {})", shim.name, shim.package),
                path: shim.path,
                size,
                name: None,
                version: None,
            });
        }
    }

    Ok(CleanupCategory {
        kind: CleanupKind::BrokenShims,
        title: "Broken shims",
        items,
    })
}

fn collect_orphaned_dependencies(installed: &[InstalledPackage]) -> Result<CleanupCategory> {
    let mut referenced: HashSet<(String, String)> = HashSet::new();
    for pkg in installed {
        for dep in &pkg.runtime_dependencies {
            referenced.insert((dep.name.clone(), dep.version.clone()));
        }
    }

    let mut items = Vec::new();
    for pkg in installed {
        if pkg.installed_as_dependency
            && !referenced.contains(&(pkg.name.clone(), pkg.version.clone()))
        {
            let size = dir_size(&pkg.cellar_path);
            items.push(CleanupItem {
                label: format!("{} {}", pkg.name, pkg.version),
                path: pkg.cellar_path.clone(),
                size,
                name: Some(pkg.name.clone()),
                version: Some(pkg.version.clone()),
            });
        }
    }

    Ok(CleanupCategory {
        kind: CleanupKind::OrphanedDependencies,
        title: "Orphaned dependencies",
        items,
    })
}

fn collect_unused_taps(
    installed: &[InstalledPackage],
    tap_manager: &TapManager,
) -> Result<CleanupCategory> {
    let mut used_taps = HashSet::new();
    for pkg in installed {
        if let Some(tap) = normalize_tap_name(&pkg.tap) {
            used_taps.insert(tap);
        }
    }

    let mut items = Vec::new();
    for tap in tap_manager.list()? {
        let full_name = tap.full_name();
        if !used_taps.contains(&full_name) {
            let size = dir_size(&tap.path);
            items.push(CleanupItem {
                label: full_name.clone(),
                path: tap.path,
                size,
                name: Some(full_name),
                version: None,
            });
        }
    }

    Ok(CleanupCategory {
        kind: CleanupKind::UnusedTaps,
        title: "Unused taps",
        items,
    })
}

fn collect_orphaned_store(db: &Database, store: &Store) -> Result<CleanupCategory> {
    let mut items = Vec::new();

    // Try to connect to the database; if it doesn't exist, there are no orphans
    let conn = match db.connect() {
        Ok(conn) => conn,
        Err(_) => {
            return Ok(CleanupCategory {
                kind: CleanupKind::OrphanedStore,
                title: "Orphaned store entries",
                items,
            });
        }
    };

    let orphaned = db.list_orphaned_store_entries(&conn)?;

    for entry in orphaned {
        let entry_path = store.entry_path(&entry.sha256);
        // Only include entries that actually exist on disk
        if entry_path.exists() {
            // Get actual size from disk (may differ from DB if modified)
            let size = dir_size(&entry_path);
            let short_sha = if entry.sha256.len() > 12 {
                format!("{}...", &entry.sha256[..12])
            } else {
                entry.sha256.clone()
            };

            items.push(CleanupItem {
                label: short_sha,
                path: entry_path,
                size,
                name: Some(entry.sha256),
                version: None,
            });
        }
    }

    Ok(CleanupCategory {
        kind: CleanupKind::OrphanedStore,
        title: "Orphaned store entries",
        items,
    })
}

fn normalize_tap_name(tap: &str) -> Option<String> {
    let parts: Vec<&str> = tap.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let user = parts[0];
    let repo = parts[1];
    let repo = if repo.starts_with("homebrew-") {
        repo.to_string()
    } else {
        format!("homebrew-{}", repo)
    };

    Some(format!("{}/{}", user, repo))
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if !path.exists() {
        return 0;
    }

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total += metadata.len();
            }
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_paths() -> (TempDir, Paths) {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        paths.init().unwrap();
        (temp, paths)
    }

    #[test]
    fn test_collect_orphaned_store_empty() {
        let (_temp, paths) = setup_paths();
        let db = Database::new(paths.clone());
        let store = Store::new(paths);

        let category = collect_orphaned_store(&db, &store).unwrap();
        assert_eq!(category.kind, CleanupKind::OrphanedStore);
        assert!(category.items.is_empty());
    }

    #[test]
    fn test_collect_orphaned_store_with_orphan() {
        let (_temp, paths) = setup_paths();
        let db = Database::new(paths.clone());
        let store = Store::new(paths.clone());

        // Create an orphaned store entry in DB
        let conn = db.connect().unwrap();
        db.upsert_store_entry(&conn, "abc123def456", 1000).unwrap();

        // Create the actual directory on disk
        let entry_path = paths.store_entry("abc123def456");
        fs::create_dir_all(&entry_path).unwrap();
        fs::write(entry_path.join("test.txt"), "test content").unwrap();

        let category = collect_orphaned_store(&db, &store).unwrap();
        assert_eq!(category.items.len(), 1);
        assert_eq!(category.items[0].label, "abc123def456");
        assert!(category.items[0].size > 0);
    }

    #[test]
    fn test_collect_orphaned_store_skips_referenced() {
        let (_temp, paths) = setup_paths();
        let db = Database::new(paths.clone());
        let store = Store::new(paths.clone());

        // Create a referenced store entry
        let conn = db.connect().unwrap();
        db.upsert_store_entry(&conn, "referenced123", 1000).unwrap();
        db.add_store_ref(&conn, "referenced123", "jq", "1.7.1")
            .unwrap();

        // Create the directory on disk
        let entry_path = paths.store_entry("referenced123");
        fs::create_dir_all(&entry_path).unwrap();
        fs::write(entry_path.join("test.txt"), "test").unwrap();

        let category = collect_orphaned_store(&db, &store).unwrap();
        assert!(category.items.is_empty());
    }

    #[test]
    fn test_collect_orphaned_store_skips_missing_on_disk() {
        let (_temp, paths) = setup_paths();
        let db = Database::new(paths.clone());
        let store = Store::new(paths);

        // Create an orphaned entry in DB but not on disk
        let conn = db.connect().unwrap();
        db.upsert_store_entry(&conn, "missing_on_disk", 1000)
            .unwrap();

        let category = collect_orphaned_store(&db, &store).unwrap();
        // Should be empty because the entry doesn't exist on disk
        assert!(category.items.is_empty());
    }

    #[test]
    fn test_apply_cleanup_orphaned_store_dry_run() {
        let (_temp, paths) = setup_paths();
        let db = Database::new(paths.clone());

        // Create an orphaned store entry
        let conn = db.connect().unwrap();
        db.upsert_store_entry(&conn, "orphan_sha256", 500).unwrap();

        // Create the directory on disk
        let entry_path = paths.store_entry("orphan_sha256");
        fs::create_dir_all(&entry_path).unwrap();
        fs::write(entry_path.join("file.bin"), "data").unwrap();

        let categories = collect_categories(&paths).unwrap();
        let mut selected = HashSet::new();
        selected.insert(CleanupKind::OrphanedStore);

        // Dry run should not delete anything
        let result = apply_cleanup(&paths, &categories, &selected, true).unwrap();
        assert_eq!(result.removed, 1);
        assert!(result.freed > 0);

        // Entry should still exist on disk
        assert!(entry_path.exists());

        // Entry should still be in DB
        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert_eq!(orphans.len(), 1);
    }

    #[test]
    fn test_apply_cleanup_orphaned_store_actual() {
        let (_temp, paths) = setup_paths();
        let db = Database::new(paths.clone());

        // Create an orphaned store entry
        let conn = db.connect().unwrap();
        db.upsert_store_entry(&conn, "to_delete_sha", 500).unwrap();

        // Create the directory on disk
        let entry_path = paths.store_entry("to_delete_sha");
        fs::create_dir_all(&entry_path).unwrap();
        fs::write(entry_path.join("file.bin"), "data").unwrap();

        let categories = collect_categories(&paths).unwrap();
        let mut selected = HashSet::new();
        selected.insert(CleanupKind::OrphanedStore);

        // Actual cleanup
        let result = apply_cleanup(&paths, &categories, &selected, false).unwrap();
        assert_eq!(result.removed, 1);
        assert!(result.freed > 0);

        // Entry should be deleted from disk
        assert!(!entry_path.exists());

        // Entry should be removed from DB
        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert!(orphans.is_empty());
    }
}
