//! Package uninstallation

use crate::cli::output::Output;
use crate::config::GlobalConfig;
use crate::error::{ColdbrewError, Result};
use crate::storage::{Cellar, Database, Paths, ShimManager};
use console::style;
use dialoguer::Confirm;
use std::collections::HashSet;
use std::io::{self, Write};

/// Uninstall a package
pub async fn uninstall(
    paths: &Paths,
    name: &str,
    version: Option<&str>,
    all: bool,
    with_deps: bool,
    yes: bool,
    output: &Output,
) -> Result<Vec<(String, String)>> {
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());

    let versions = cellar.get_versions(name)?;

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

        if let Some(sha256) = bottle_sha {
            let db = Database::new(paths.clone());
            let conn = db.connect()?;
            db.remove_store_ref(&conn, &sha256, name, version)?;
        }

        removed.push((name.to_string(), version.clone()));
    }

    // Handle --with-deps: find and remove orphan dependencies
    if with_deps {
        output.debug("Checking for orphan dependencies...");

        let orphans = find_orphan_dependencies(paths, name)?;

        if !orphans.is_empty() {
            output.info("The following dependencies are no longer required:");
            for orphan in &orphans {
                println!(
                    "  {} {}",
                    style("•").dim(),
                    Output::package_name(&orphan.0)
                );
            }

            let should_remove = if yes {
                true
            } else {
                // Prompt user for confirmation
                println!();
                io::stdout().flush().ok();
                Confirm::new()
                    .with_prompt("Remove these orphan dependencies?")
                    .default(false)
                    .interact()
                    .unwrap_or(false)
            };

            if should_remove {
                for (orphan_name, orphan_version) in &orphans {
                    output.debug(&format!("Removing orphan {} {}...", orphan_name, orphan_version));

                    // Remove shims
                    let binaries = cellar.get_binaries(orphan_name, orphan_version)?;
                    if !binaries.is_empty() {
                        shim_manager.remove_shims(&binaries)?;
                    }

                    // Get bottle SHA before removal
                    let bottle_sha = cellar
                        .get_package(orphan_name, orphan_version)
                        .ok()
                        .and_then(|pkg| pkg.bottle_sha256.clone());

                    // Remove from cellar
                    cellar.uninstall(orphan_name, orphan_version)?;

                    // Clean up store reference
                    if let Some(sha256) = bottle_sha {
                        let db = Database::new(paths.clone());
                        let conn = db.connect()?;
                        db.remove_store_ref(&conn, &sha256, orphan_name, orphan_version)?;
                    }

                    // Remove from config
                    let mut config = GlobalConfig::load(paths)?;
                    config.remove_default(orphan_name);
                    config.remove_pin(orphan_name);
                    config.save(paths)?;

                    removed.push((orphan_name.clone(), orphan_version.clone()));
                }
            } else {
                output.info("Skipped removal of orphan dependencies");
            }
        } else {
            output.debug("No orphan dependencies found");
        }
    }

    Ok(removed)
}

/// Find dependencies that are no longer required by any installed package
///
/// A dependency is considered an orphan if:
/// 1. It was installed as a dependency (installed_as_dependency = true)
/// 2. No other installed package (that isn't itself an orphan) depends on it
///
/// This function performs transitive orphan detection - if A depends on B,
/// and A becomes an orphan, B may also become an orphan if nothing else needs it.
pub fn find_orphan_dependencies(
    paths: &Paths,
    uninstalled_package: &str,
) -> Result<Vec<(String, String)>> {
    let cellar = Cellar::new(paths.clone());
    let installed = cellar.list_packages()?;

    // Build a map of package name -> package for quick lookup
    let pkg_map: std::collections::HashMap<String, _> = installed
        .iter()
        .map(|pkg| (pkg.name.clone(), pkg))
        .collect();

    // Start with packages to exclude (being uninstalled)
    let mut excluded: HashSet<String> = HashSet::new();
    excluded.insert(uninstalled_package.to_string());

    // Iteratively find orphans until no new ones are found
    loop {
        let mut new_orphans: HashSet<String> = HashSet::new();

        // For each package installed as a dependency
        for pkg in installed.iter().filter(|p| p.installed_as_dependency) {
            if excluded.contains(&pkg.name) {
                continue; // Already marked as orphan or being uninstalled
            }

            // Check if any non-excluded package depends on this one
            let is_still_required = installed.iter().any(|other| {
                // Skip excluded packages (being uninstalled or already orphaned)
                if excluded.contains(&other.name) {
                    return false;
                }
                // Skip self
                if other.name == pkg.name {
                    return false;
                }
                // Check if other depends on this package
                other
                    .runtime_dependencies
                    .iter()
                    .any(|dep| dep.name == pkg.name)
            });

            if !is_still_required {
                new_orphans.insert(pkg.name.clone());
            }
        }

        if new_orphans.is_empty() {
            break; // No more orphans found
        }

        // Add new orphans to excluded set for next iteration
        excluded.extend(new_orphans);
    }

    // Remove the originally uninstalled package from excluded set to get just orphans
    excluded.remove(uninstalled_package);

    // Collect orphan packages with their versions
    let orphans: Vec<(String, String)> = excluded
        .into_iter()
        .filter_map(|name| pkg_map.get(&name).map(|pkg| (name, pkg.version.clone())))
        .collect();

    Ok(orphans)
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
    use super::*;
    use crate::core::package::{InstalledPackage, PackageMetadata, RuntimeDependency};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_package(
        name: &str,
        version: &str,
        deps: Vec<(&str, &str)>,
        as_dependency: bool,
    ) -> InstalledPackage {
        let mut pkg = InstalledPackage::new(
            name.to_string(),
            version.to_string(),
            "homebrew/core".to_string(),
            PathBuf::from(format!("/test/cellar/{}/{}", name, version)),
        );
        pkg.installed_as_dependency = as_dependency;
        pkg.runtime_dependencies = deps
            .into_iter()
            .map(|(n, v)| RuntimeDependency {
                name: n.to_string(),
                version: v.to_string(),
                path: PathBuf::from(format!("/test/cellar/{}/{}", n, v)),
            })
            .collect();
        pkg
    }

    fn setup_test_cellar(temp: &TempDir, packages: &[InstalledPackage]) -> Paths {
        let paths = Paths::with_root(temp.path().to_path_buf());
        let cellar = Cellar::new(paths.clone());

        for pkg in packages {
            // Create package directory
            let pkg_dir = paths.cellar_package(&pkg.name, &pkg.version);
            std::fs::create_dir_all(&pkg_dir).unwrap();

            // Save metadata
            let metadata = PackageMetadata::new(pkg.clone(), "test://source".to_string());
            cellar.save_metadata(&metadata).unwrap();
        }

        paths
    }

    #[test]
    fn test_find_orphans_no_deps() {
        let temp = TempDir::new().unwrap();

        // jq with no dependencies
        let jq = create_test_package("jq", "1.7.1", vec![], false);
        let paths = setup_test_cellar(&temp, &[jq]);

        let orphans = find_orphan_dependencies(&paths, "jq").unwrap();
        assert!(orphans.is_empty(), "Should find no orphans when no deps exist");
    }

    #[test]
    fn test_find_orphans_single_orphan() {
        let temp = TempDir::new().unwrap();

        // jq depends on oniguruma (installed as dependency)
        let jq = create_test_package("jq", "1.7.1", vec![("oniguruma", "6.9.8")], false);
        let oniguruma = create_test_package("oniguruma", "6.9.8", vec![], true);

        let paths = setup_test_cellar(&temp, &[jq, oniguruma]);

        // After uninstalling jq, oniguruma should be an orphan
        let orphans = find_orphan_dependencies(&paths, "jq").unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].0, "oniguruma");
    }

    #[test]
    fn test_find_orphans_shared_dependency() {
        let temp = TempDir::new().unwrap();

        // Both jq and ripgrep depend on pcre2
        let jq = create_test_package("jq", "1.7.1", vec![("pcre2", "10.42")], false);
        let ripgrep = create_test_package("ripgrep", "14.0.0", vec![("pcre2", "10.42")], false);
        let pcre2 = create_test_package("pcre2", "10.42", vec![], true);

        let paths = setup_test_cellar(&temp, &[jq, ripgrep, pcre2]);

        // After uninstalling jq, pcre2 is still needed by ripgrep
        let orphans = find_orphan_dependencies(&paths, "jq").unwrap();
        assert!(orphans.is_empty(), "pcre2 should not be orphaned - still needed by ripgrep");
    }

    #[test]
    fn test_find_orphans_transitive_deps() {
        let temp = TempDir::new().unwrap();

        // ffmpeg -> x264 -> libmp3lame (all as dependencies)
        let ffmpeg = create_test_package("ffmpeg", "6.0", vec![("x264", "164")], false);
        let x264 = create_test_package("x264", "164", vec![("libmp3lame", "3.100")], true);
        let libmp3lame = create_test_package("libmp3lame", "3.100", vec![], true);

        let paths = setup_test_cellar(&temp, &[ffmpeg, x264, libmp3lame]);

        // After uninstalling ffmpeg, both x264 and libmp3lame become orphans
        let orphans = find_orphan_dependencies(&paths, "ffmpeg").unwrap();
        assert_eq!(orphans.len(), 2);

        let orphan_names: HashSet<_> = orphans.iter().map(|(n, _)| n.as_str()).collect();
        assert!(orphan_names.contains("x264"));
        assert!(orphan_names.contains("libmp3lame"));
    }

    #[test]
    fn test_find_orphans_explicitly_installed() {
        let temp = TempDir::new().unwrap();

        // jq depends on oniguruma, but oniguruma was installed explicitly
        let jq = create_test_package("jq", "1.7.1", vec![("oniguruma", "6.9.8")], false);
        let oniguruma = create_test_package("oniguruma", "6.9.8", vec![], false); // NOT as dependency

        let paths = setup_test_cellar(&temp, &[jq, oniguruma]);

        // oniguruma should NOT be an orphan because it was installed explicitly
        let orphans = find_orphan_dependencies(&paths, "jq").unwrap();
        assert!(orphans.is_empty(), "Explicitly installed packages should not be orphaned");
    }

    #[tokio::test]
    async fn test_check_dependents() {
        let temp = TempDir::new().unwrap();

        let jq = create_test_package("jq", "1.7.1", vec![("oniguruma", "6.9.8")], false);
        let oniguruma = create_test_package("oniguruma", "6.9.8", vec![], true);

        let paths = setup_test_cellar(&temp, &[jq, oniguruma]);

        let dependents = check_dependents(&paths, "oniguruma").await.unwrap();
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], "jq");
    }
}
