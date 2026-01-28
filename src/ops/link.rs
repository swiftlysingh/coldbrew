//! Package linking operations

use crate::error::{ColdbrewError, Result};
use crate::storage::{Cellar, Paths, ShimManager};

/// Link a package (create shims)
pub fn link(paths: &Paths, name: &str, version: &str, force: bool) -> Result<Vec<String>> {
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());

    // Check if package exists
    if !cellar.is_installed(name, version) {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.to_string(),
            version: version.to_string(),
        });
    }

    // Get binaries
    let binaries = cellar.get_binaries(name, version)?;

    if binaries.is_empty() {
        return Ok(Vec::new());
    }

    // Check for existing shims
    if !force {
        for binary in &binaries {
            if shim_manager.has_shim(binary) {
                return Err(ColdbrewError::Other(format!(
                    "Shim for '{}' already exists. Use --force to overwrite",
                    binary
                )));
            }
        }
    }

    // Create shims
    shim_manager.create_shims(name, version, &binaries)?;

    Ok(binaries)
}

/// Unlink a package (remove shims)
pub fn unlink(paths: &Paths, name: &str, version: &str) -> Result<Vec<String>> {
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());

    // Get binaries
    let binaries = cellar.get_binaries(name, version)?;

    if binaries.is_empty() {
        return Ok(Vec::new());
    }

    // Remove shims
    shim_manager.remove_shims(&binaries)?;

    Ok(binaries)
}

/// Relink all packages (useful after upgrades)
pub fn relink_all(paths: &Paths) -> Result<usize> {
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());

    let packages = cellar.list_packages()?;
    let mut linked = 0;

    for pkg in packages {
        if pkg.keg_only {
            continue;
        }

        let binaries = cellar.get_binaries(&pkg.name, &pkg.version)?;
        if !binaries.is_empty() {
            shim_manager.create_shims(&pkg.name, &pkg.version, &binaries)?;
            linked += 1;
        }
    }

    Ok(linked)
}
