//! Package uninstallation

use crate::cli::output::Output;
use crate::config::GlobalConfig;
use crate::error::{ColdbrewError, Result};
use crate::storage::{Cellar, Paths, ShimManager};

/// Uninstall a package
pub async fn uninstall(
    paths: &Paths,
    name: &str,
    version: Option<&str>,
    all: bool,
    with_deps: bool,
    output: &Output,
) -> Result<Vec<(String, String)>> {
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());

    let versions = cellar.get_versions(name)?;

    if versions.is_empty() {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.to_string(),
            version: version.map(String::from).unwrap_or_else(|| "any".to_string()),
        });
    }

    let versions_to_remove: Vec<String> = if all {
        versions
    } else if let Some(v) = version {
        if !versions.contains(&v.to_string()) {
            return Err(ColdbrewError::PackageNotInstalled {
                name: name.to_string(),
                version: v.to_string(),
            });
        }
        vec![v.to_string()]
    } else {
        // Remove latest version by default
        vec![versions.last().unwrap().clone()]
    };

    let mut removed = Vec::new();

    for version in &versions_to_remove {
        // Get binaries before removal
        let binaries = cellar.get_binaries(name, version)?;

        // Remove shims if this is the last version
        let remaining_versions: Vec<_> = versions
            .iter()
            .filter(|v| !versions_to_remove.contains(v))
            .collect();

        if remaining_versions.is_empty() {
            output.debug(&format!("Removing shims for {}", name));
            shim_manager.remove_shims(&binaries)?;

            // Remove from defaults
            let mut config = GlobalConfig::load(paths)?;
            config.remove_default(name);
            config.remove_pin(name);
            config.save(paths)?;
        }

        // Remove from cellar
        output.debug(&format!("Removing {} {}...", name, version));
        cellar.uninstall(name, version)?;

        removed.push((name.to_string(), version.clone()));
    }

    // TODO: Handle with_deps - remove orphan dependencies
    if with_deps {
        output.debug("Checking for orphan dependencies...");
        // This would require tracking which packages were installed as dependencies
        // and checking if they're still needed by other packages
    }

    Ok(removed)
}

/// Check if a package can be safely uninstalled (no dependents)
pub async fn check_dependents(paths: &Paths, name: &str) -> Result<Vec<String>> {
    let cellar = Cellar::new(paths.clone());

    // Get all installed packages
    let installed = cellar.list_packages()?;

    // Find packages that depend on this one
    let mut dependents = Vec::new();
    for pkg in installed {
        for dep in &pkg.runtime_dependencies {
            if dep.name == name {
                dependents.push(pkg.name.clone());
                break;
            }
        }
    }

    Ok(dependents)
}
