use crate::error::{ColdbrewError, Result};
use crate::storage::paths::Paths;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct StoreEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created: bool,
}

/// Content-addressable store for extracted bottles
pub struct Store {
    paths: Paths,
}

impl Store {
    /// Create a new store manager
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    /// Ensure a store entry exists for the given sha
    pub fn ensure_entry(&self, sha256: &str, bottle_path: &Path) -> Result<StoreEntry> {
        let entry_path = self.paths.store_entry(sha256);
        if entry_path.exists() {
            let size_bytes = dir_size(&entry_path)?;
            return Ok(StoreEntry {
                path: entry_path,
                size_bytes,
                created: false,
            });
        }

        if let Some(parent) = entry_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let lock_path = self.paths.store_lock(sha256);
        let _lock = StoreLock::acquire(&lock_path, &entry_path)?;

        if entry_path.exists() {
            let size_bytes = dir_size(&entry_path)?;
            return Ok(StoreEntry {
                path: entry_path,
                size_bytes,
                created: false,
            });
        }

        fs::create_dir_all(&entry_path)?;
        if let Err(err) = extract_bottle(bottle_path, &entry_path) {
            let _ = fs::remove_dir_all(&entry_path);
            return Err(err);
        }

        let size_bytes = dir_size(&entry_path)?;
        Ok(StoreEntry {
            path: entry_path,
            size_bytes,
            created: true,
        })
    }

    /// Check if a store entry already exists
    pub fn entry_exists(&self, sha256: &str) -> bool {
        self.paths.store_entry(sha256).exists()
    }

    /// Get the size of an existing store entry
    pub fn entry_size(&self, sha256: &str) -> Result<u64> {
        let entry_path = self.paths.store_entry(sha256);
        if !entry_path.exists() {
            return Err(ColdbrewError::PathNotFound(entry_path));
        }
        dir_size(&entry_path)
    }

    /// Materialize a store entry into the cellar
    pub fn materialize(&self, sha256: &str, name: &str, version: &str) -> Result<PathBuf> {
        let entry_path = self.paths.store_entry(sha256);
        if !entry_path.exists() {
            return Err(ColdbrewError::PathNotFound(entry_path));
        }

        let target_dir = self.paths.cellar_package(name, version);
        if target_dir.exists() {
            return Err(ColdbrewError::PackageAlreadyInstalled {
                name: name.to_string(),
                version: version.to_string(),
            });
        }

        if let Some(parent) = target_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::create_dir_all(&target_dir)?;
        copy_tree(&entry_path, &target_dir)?;
        Ok(target_dir)
    }
}

struct StoreLock {
    path: PathBuf,
    owned: bool,
}

impl StoreLock {
    fn acquire(lock_path: &Path, entry_path: &Path) -> Result<Self> {
        let start = Instant::now();
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(lock_path)
            {
                Ok(_) => {
                    return Ok(Self {
                        path: lock_path.to_path_buf(),
                        owned: true,
                    })
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    if entry_path.exists() {
                        return Ok(Self {
                            path: lock_path.to_path_buf(),
                            owned: false,
                        });
                    }
                    if start.elapsed() > Duration::from_secs(30) {
                        return Err(ColdbrewError::Other(
                            "Timed out waiting for store lock".to_string(),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
                Err(err) => return Err(err.into()),
            }
        }
    }
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        if self.owned {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn extract_bottle(bottle_path: &Path, target_dir: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    fn strip_components(path: &Path, skip: usize) -> Result<PathBuf> {
        let mut relative_path = PathBuf::new();
        let mut skipped = 0;

        for component in path.components() {
            if skipped < skip {
                skipped += 1;
                continue;
            }

            match component {
                Component::Normal(part) => relative_path.push(part),
                _ => {
                    return Err(ColdbrewError::ExtractionFailed(format!(
                        "Invalid bottle entry path: {}",
                        path.display()
                    )));
                }
            }
        }

        Ok(relative_path)
    }

    let file = fs::File::open(bottle_path)
        .map_err(|err| ColdbrewError::ExtractionFailed(err.to_string()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let entries = archive
        .entries()
        .map_err(|err| ColdbrewError::ExtractionFailed(err.to_string()))?;

    let mut pending_links: Vec<(PathBuf, PathBuf)> = Vec::new();

    for entry in entries {
        let mut entry = entry.map_err(|err| ColdbrewError::ExtractionFailed(err.to_string()))?;
        let path = entry
            .path()
            .map_err(|err| ColdbrewError::ExtractionFailed(err.to_string()))?;
        let relative_path = strip_components(&path, 2)?;

        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let dest = target_dir.join(&relative_path);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        if entry.header().entry_type().is_hard_link() {
            let link_name = entry
                .link_name()
                .map_err(|err| ColdbrewError::ExtractionFailed(err.to_string()))?
                .ok_or_else(|| {
                    ColdbrewError::ExtractionFailed("Hard link missing target".to_string())
                })?;
            let link_relative = strip_components(&link_name, 2)?;
            if link_relative.as_os_str().is_empty() {
                return Err(ColdbrewError::ExtractionFailed(
                    "Hard link target is empty".to_string(),
                ));
            }
            let link_target = target_dir.join(&link_relative);
            if dest.exists() {
                continue;
            }
            if link_target.exists() {
                fs::hard_link(&link_target, &dest)
                    .map_err(|err| ColdbrewError::ExtractionFailed(err.to_string()))?;
            } else {
                pending_links.push((dest, link_target));
            }
            continue;
        }

        entry
            .unpack(&dest)
            .map_err(|err| ColdbrewError::ExtractionFailed(err.to_string()))?;
    }

    for (dest, link_target) in pending_links {
        if dest.exists() {
            continue;
        }
        if !link_target.exists() {
            return Err(ColdbrewError::ExtractionFailed(format!(
                "Hard link target missing: {}",
                link_target.display()
            )));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::hard_link(&link_target, &dest)
            .map_err(|err| ColdbrewError::ExtractionFailed(err.to_string()))?;
    }

    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|err| ColdbrewError::Other(err.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source)
            .map_err(|err| ColdbrewError::Other(err.to_string()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }

        if entry.file_type().is_symlink() {
            let link_target = fs::read_link(path)?;
            create_symlink(&link_target, &target)?;
            continue;
        }

        if entry.file_type().is_file() {
            copy_with_fallback(path, &target)?;
        }
    }

    Ok(())
}

fn copy_with_fallback(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    if try_clonefile(source, destination).is_ok() {
        return Ok(());
    }

    if fs::hard_link(source, destination).is_ok() {
        return Ok(());
    }

    fs::copy(source, destination)?;
    let permissions = fs::metadata(source)?.permissions();
    fs::set_permissions(destination, permissions)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn try_clonefile(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let source_c = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid source path"))?;
    let dest_c = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid dest path"))?;

    let result = unsafe { libc::clonefile(source_c.as_ptr(), dest_c.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(target_os = "macos"))]
fn try_clonefile(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "clonefile not supported",
    ))
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<()> {
    Err(ColdbrewError::Other(
        "Symlinks not supported on this platform".to_string(),
    ))
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0;
    for entry in WalkDir::new(path) {
        let entry = entry.map_err(|err| ColdbrewError::Other(err.to_string()))?;
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}
