//! Package uninstallation

use crate::cli::output::Output;
use crate::config::GlobalConfig;
use crate::error::{ColdbrewError, Result};
use crate::storage::{Cellar, Database, Paths, ShimManager};
use std::collections::HashSet;

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
    let installed_before = cellar.list_packages()?;

    if versions.is_empty() {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.to_string(),
            version: version
                .map(String::from)
                .unwrap_or_else(|| "any".to_string()),
        });
    }

    let versions_to_remove: Vec<String> = if all {
        versions.clone()
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
        let bottle_sha = cellar
            .get_package(name, version)
            .ok()
            .and_then(|pkg| pkg.bottle_sha256.clone());

        // Get binaries before removal
        let binaries = cellar.get_binaries(name, version)?;

        // Remove shims if this is the last version
        let remaining_versions: Vec<_> = versions
            .iter()
            .filter(|v| !versions_to_remove.contains(v))
            .collect();

        if remaining_versions.is_empty() {
            output.debug(&format!("Removing shims for {}", name));
            shim_manager.remove_shims(name, &binaries)?;

            // Remove from defaults
            let mut config = GlobalConfig::load(paths)?;
            config.remove_default(name);
            config.remove_pin(name);
            config.save(paths)?;
        }

        // Remove from cellar
        output.debug(&format!("Removing {} {}...", name, version));
        cellar.uninstall(name, version)?;

        if let Some(sha256) = bottle_sha {
            let db = Database::new(paths.clone());
            let conn = db.connect()?;
            db.remove_store_ref(&conn, &sha256, name, version)?;
        }

        removed.push((name.to_string(), version.clone()));
    }

    if with_deps {
        output.debug("Checking for orphan dependencies...");
        let removed_set: HashSet<_> = removed.iter().cloned().collect();
        let orphans = find_orphan_dependencies(&installed_before, &removed_set);
        for (orphan_name, orphan_version) in orphans {
            let orphan_versions = cellar.get_versions(&orphan_name)?;
            let binaries = cellar.get_binaries(&orphan_name, &orphan_version)?;
            if orphan_versions.len() == 1 {
                shim_manager.remove_shims(&orphan_name, &binaries)?;
                let mut config = GlobalConfig::load(paths)?;
                config.remove_default(&orphan_name);
                config.remove_pin(&orphan_name);
                config.save(paths)?;
            }
            let bottle_sha = cellar
                .get_package(&orphan_name, &orphan_version)
                .ok()
                .and_then(|pkg| pkg.bottle_sha256);
            cellar.uninstall(&orphan_name, &orphan_version)?;
            if let Some(sha256) = bottle_sha {
                let db = Database::new(paths.clone());
                let conn = db.connect()?;
                db.remove_store_ref(&conn, &sha256, &orphan_name, &orphan_version)?;
            }
            removed.push((orphan_name, orphan_version));
        }
    }

    Ok(removed)
}

pub fn find_orphan_dependencies(
    installed: &[crate::core::package::InstalledPackage],
    removed_packages: &HashSet<(String, String)>,
) -> Vec<(String, String)> {
    let mut excluded = removed_packages.clone();
    let mut candidates: HashSet<_> = installed
        .iter()
        .filter(|pkg| removed_packages.contains(&(pkg.name.clone(), pkg.version.clone())))
        .flat_map(|pkg| pkg.runtime_dependencies.iter())
        .map(|dep| (dep.name.clone(), dep.version.clone()))
        .collect();
    loop {
        let new_orphans: HashSet<_> = installed
            .iter()
            .filter(|pkg| pkg.installed_as_dependency)
            .filter(|pkg| candidates.contains(&(pkg.name.clone(), pkg.version.clone())))
            .filter(|pkg| !excluded.contains(&(pkg.name.clone(), pkg.version.clone())))
            .filter(|pkg| !installed.iter().any(|other| {
                !excluded.contains(&(other.name.clone(), other.version.clone()))
                    && (other.name != pkg.name || other.version != pkg.version)
                    && other.runtime_dependencies.iter().any(|dep| {
                        dep.name == pkg.name && dep.version == pkg.version
                    })
            }))
            .map(|pkg| (pkg.name.clone(), pkg.version.clone()))
            .collect();
        if new_orphans.is_empty() { break; }
        for orphan in &new_orphans {
            if let Some(pkg) = installed.iter().find(|pkg| (pkg.name.clone(), pkg.version.clone()) == *orphan) {
                candidates.extend(pkg.runtime_dependencies.iter().map(|dep| (dep.name.clone(), dep.version.clone())));
            }
        }
        excluded.extend(new_orphans);
    }
    let mut result: Vec<_> = excluded
        .into_iter()
        .filter(|p| !removed_packages.contains(p))
        .collect();
    result.sort();
    result
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

#[cfg(test)]
mod tests {
    use super::find_orphan_dependencies;
    use crate::core::package::{InstalledPackage, RuntimeDependency};
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn package(name: &str, dependency: bool, deps: &[(&str, &str)]) -> InstalledPackage {
        let mut package = InstalledPackage::new(
            name.to_string(),
            "1.0".to_string(),
            "homebrew/core".to_string(),
            PathBuf::from(format!("/cellar/{name}/1.0")),
        );
        package.installed_as_dependency = dependency;
        package.runtime_dependencies = deps
            .iter()
            .map(|(name, version)| RuntimeDependency {
                name: (*name).to_string(),
                version: (*version).to_string(),
                path: PathBuf::new(),
            })
            .collect();
        package
    }

    fn removed(name: &str) -> HashSet<(String, String)> {
        HashSet::from([(name.to_string(), "1.0".to_string())])
    }

    #[test]
    fn removes_transitive_dependency_chain() {
        let installed = vec![
            package("app", false, &[("dep", "1.0")]),
            package("dep", true, &[("nested", "1.0")]),
            package("nested", true, &[]),
        ];
        assert_eq!(
            find_orphan_dependencies(&installed, &removed("app")),
            vec![
                ("dep".into(), "1.0".into()),
                ("nested".into(), "1.0".into())
            ]
        );
    }

    #[test]
    fn preserves_shared_dependency() {
        let installed = vec![
            package("app", false, &[("dep", "1.0")]),
            package("other", false, &[("dep", "1.0")]),
            package("dep", true, &[]),
        ];
        assert!(find_orphan_dependencies(&installed, &removed("app")).is_empty());
    }

    #[test]
    fn preserves_explicit_dependency() {
        let installed = vec![
            package("app", false, &[("dep", "1.0")]),
            package("dep", false, &[]),
        ];
        assert!(find_orphan_dependencies(&installed, &removed("app")).is_empty());
    }

    #[test]
    fn preserves_unrelated_preexisting_orphan() {
        let installed = vec![
            package("app", false, &[("dep", "1.0")]),
            package("dep", true, &[]),
            package("old-orphan", true, &[]),
        ];
        assert_eq!(
            find_orphan_dependencies(&installed, &removed("app")),
            vec![("dep".into(), "1.0".into())]
        );
    }
}
