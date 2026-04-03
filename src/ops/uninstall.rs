//! Package uninstallation

use crate::cli::output::Output;
use crate::config::GlobalConfig;
use crate::error::{ColdbrewError, Result};
use crate::storage::{Cellar, Database, Paths, ShimManager};
use console::style;
use dialoguer::Confirm;
use rusqlite::Connection;
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
    let db = Database::new(paths.clone());
    let conn = db.connect()?;

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

    let mut config = GlobalConfig::load(paths)?;
    let mut removed = Vec::new();
    let removal = PackageRemoval {
        cellar: &cellar,
        shim_manager: &shim_manager,
        db: &db,
        conn: &conn,
        output,
    };

    for version in &versions_to_remove {
        let remaining_versions: Vec<_> = versions
            .iter()
            .filter(|v| !versions_to_remove.contains(v))
            .collect();

        output.debug(&format!("Removing {} {}...", name, version));
        remove_single_package(&removal, name, version, remaining_versions.is_empty())?;

        if remaining_versions.is_empty() {
            config.remove_default(name);
            config.remove_pin(name);
        }

        removed.push((name.to_string(), version.clone()));
    }

    let removed_set: HashSet<(String, String)> = removed.iter().cloned().collect();

    // Handle --with-deps: find and remove orphan dependencies
    if with_deps {
        output.debug("Checking for orphan dependencies...");

        // The primary package versions have already been removed from the cellar,
        // so pass the exact removed versions explicitly instead of relying on the
        // current filesystem state to infer what's being uninstalled.
        let orphans = find_orphan_dependencies(paths, &removed_set)?;

        if !orphans.is_empty() {
            output.info("The following dependencies are no longer required:");
            for orphan in &orphans {
                println!(
                    "  {} {} {}",
                    style("•").dim(),
                    Output::package_name(&orphan.0),
                    Output::version(&orphan.1)
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
                    output.debug(&format!(
                        "Removing orphan {} {}...",
                        orphan_name, orphan_version
                    ));

                    let remaining_versions = cellar.get_versions(orphan_name)?;
                    let should_remove_shims = remaining_versions.len() == 1;

                    remove_single_package(
                        &removal,
                        orphan_name,
                        orphan_version,
                        should_remove_shims,
                    )?;

                    config.remove_default(orphan_name);
                    config.remove_pin(orphan_name);

                    removed.push((orphan_name.clone(), orphan_version.clone()));
                }
            } else {
                output.info("Skipped removal of orphan dependencies");
            }
        } else {
            output.debug("No orphan dependencies found");
        }
    }

    config.save(paths)?;

    Ok(removed)
}

struct PackageRemoval<'a> {
    cellar: &'a Cellar,
    shim_manager: &'a ShimManager,
    db: &'a Database,
    conn: &'a Connection,
    output: &'a Output,
}

fn remove_single_package(
    removal: &PackageRemoval<'_>,
    name: &str,
    version: &str,
    remove_shims: bool,
) -> Result<()> {
    let bottle_sha = removal
        .cellar
        .get_package(name, version)
        .ok()
        .and_then(|pkg| pkg.bottle_sha256.clone());

    if remove_shims {
        let binaries = removal.cellar.get_binaries(name, version)?;
        if !binaries.is_empty() {
            removal
                .output
                .debug(&format!("Removing shims for {}", name));
            removal.shim_manager.remove_shims(&binaries)?;
        }
    }

    removal.cellar.uninstall(name, version)?;

    if let Some(sha256) = bottle_sha {
        removal
            .db
            .remove_store_ref(removal.conn, &sha256, name, version)?;
    }

    Ok(())
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
    removed_packages: &HashSet<(String, String)>,
) -> Result<Vec<(String, String)>> {
    let cellar = Cellar::new(paths.clone());
    let installed = cellar.list_packages()?;
    let mut excluded: HashSet<(String, String)> = removed_packages.clone();

    // Iteratively find orphans until no new ones are found
    loop {
        let mut new_orphans: HashSet<(String, String)> = HashSet::new();

        // For each package installed as a dependency
        for pkg in installed.iter().filter(|p| p.installed_as_dependency) {
            let pkg_key = (pkg.name.clone(), pkg.version.clone());
            if excluded.contains(&pkg_key) {
                continue; // Already marked as orphan or being uninstalled
            }

            // Check if any non-excluded package depends on this one
            let is_still_required = installed.iter().any(|other| {
                // Skip excluded packages (being uninstalled or already orphaned)
                let other_key = (other.name.clone(), other.version.clone());
                if excluded.contains(&other_key) {
                    return false;
                }
                // Skip self
                if other.name == pkg.name && other.version == pkg.version {
                    return false;
                }
                // Check if other depends on this package
                other
                    .runtime_dependencies
                    .iter()
                    .any(|dep| dep.name == pkg.name && dep.version == pkg.version)
            });

            if !is_still_required {
                new_orphans.insert(pkg_key);
            }
        }

        if new_orphans.is_empty() {
            break; // No more orphans found
        }

        // Add new orphans to excluded set for next iteration
        excluded.extend(new_orphans);
    }

    let mut orphans: Vec<(String, String)> = excluded
        .into_iter()
        .filter(|pkg| !removed_packages.contains(pkg))
        .collect();
    orphans.sort();

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

        let removed = HashSet::from([(String::from("jq"), String::from("1.7.1"))]);
        let orphans = find_orphan_dependencies(&paths, &removed).unwrap();
        assert!(
            orphans.is_empty(),
            "Should find no orphans when no deps exist"
        );
    }

    #[test]
    fn test_find_orphans_single_orphan() {
        let temp = TempDir::new().unwrap();

        // jq depends on oniguruma (installed as dependency)
        let jq = create_test_package("jq", "1.7.1", vec![("oniguruma", "6.9.8")], false);
        let oniguruma = create_test_package("oniguruma", "6.9.8", vec![], true);

        let paths = setup_test_cellar(&temp, &[jq, oniguruma]);

        // After uninstalling jq, oniguruma should be an orphan
        let removed = HashSet::from([(String::from("jq"), String::from("1.7.1"))]);
        let orphans = find_orphan_dependencies(&paths, &removed).unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].0, "oniguruma");
        assert_eq!(orphans[0].1, "6.9.8");
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
        let removed = HashSet::from([(String::from("jq"), String::from("1.7.1"))]);
        let orphans = find_orphan_dependencies(&paths, &removed).unwrap();
        assert!(
            orphans.is_empty(),
            "pcre2 should not be orphaned - still needed by ripgrep"
        );
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
        let removed = HashSet::from([(String::from("ffmpeg"), String::from("6.0"))]);
        let orphans = find_orphan_dependencies(&paths, &removed).unwrap();
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
        let removed = HashSet::from([(String::from("jq"), String::from("1.7.1"))]);
        let orphans = find_orphan_dependencies(&paths, &removed).unwrap();
        assert!(
            orphans.is_empty(),
            "Explicitly installed packages should not be orphaned"
        );
    }

    #[test]
    fn test_find_orphans_preserves_remaining_version_dependencies() {
        let temp = TempDir::new().unwrap();

        let foo_old = create_test_package("foo", "1.0.0", vec![("bar", "2.0.0")], false);
        let mut foo_new = create_test_package("foo", "2.0.0", vec![("bar", "2.0.0")], false);
        foo_new.installed_as_dependency = false;
        let bar = create_test_package("bar", "2.0.0", vec![], true);

        let paths = setup_test_cellar(&temp, &[foo_old, foo_new, bar]);
        let removed = HashSet::from([(String::from("foo"), String::from("1.0.0"))]);

        let orphans = find_orphan_dependencies(&paths, &removed).unwrap();
        assert!(
            orphans.is_empty(),
            "Dependencies still required by another installed version must be kept"
        );
    }

    #[test]
    fn test_find_orphans_returns_all_orphan_versions() {
        let temp = TempDir::new().unwrap();

        let app = create_test_package(
            "app",
            "1.0.0",
            vec![("lib", "1.0.0"), ("lib", "2.0.0")],
            false,
        );
        let lib_v1 = create_test_package("lib", "1.0.0", vec![], true);
        let lib_v2 = create_test_package("lib", "2.0.0", vec![], true);

        let paths = setup_test_cellar(&temp, &[app, lib_v1, lib_v2]);
        let removed = HashSet::from([(String::from("app"), String::from("1.0.0"))]);

        let orphans = find_orphan_dependencies(&paths, &removed).unwrap();
        assert_eq!(
            orphans,
            vec![
                (String::from("lib"), String::from("1.0.0")),
                (String::from("lib"), String::from("2.0.0"))
            ]
        );
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
