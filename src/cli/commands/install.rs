//! Install command - install packages

use crate::cli::output::Output;
use crate::config::{Lockfile, ProjectConfig};
use crate::core::version::parse_package_spec;
use crate::error::{ColdbrewError, Result};
use crate::ops;
use crate::storage::{Cellar, Paths};
use std::env;
use std::path::{Path, PathBuf};

/// Execute the install command
pub async fn execute(
    packages: &[String],
    from_lock: bool,
    skip_deps: bool,
    force: bool,
    output: &Output,
) -> Result<()> {
    let paths = Paths::new()?;
    paths.init()?;

    if from_lock {
        return execute_from_lockfile(&paths, force, output).await;
    }

    for package in packages {
        let (name, version) = parse_package_spec(package);

        output.info(&format!(
            "Installing {}{}",
            Output::package_name(&name),
            version
                .as_ref()
                .map(|v| format!("@{}", v))
                .unwrap_or_default()
        ));

        let result =
            ops::install::install(&paths, &name, version.as_deref(), skip_deps, force, output)
                .await;

        match result {
            Ok(installed) => {
                output.success(&format!(
                    "Installed {} {}",
                    Output::package_name(&installed.name),
                    Output::version(&installed.version)
                ));

                if let Some(ref caveats) = installed.caveats {
                    output.caveats(caveats);
                }

                hint_path(&paths, &installed, output);
            }
            Err(ColdbrewError::PackageAlreadyInstalled { name, version }) => {
                output.info(&format!(
                    "Already installed {} {}",
                    Output::package_name(&name),
                    Output::version(&version)
                ));

                let cellar = Cellar::new(paths.clone());
                if let Ok(installed) = cellar.get_package(&name, &version) {
                    hint_path(&paths, &installed, output);
                }
            }
            Err(e) => {
                output.error(&format!("Failed to install {}: {}", name, e));
                if let Some(suggestion) = e.suggestion() {
                    output.hint(suggestion);
                }
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Execute install from lockfile
async fn execute_from_lockfile(paths: &Paths, force: bool, output: &Output) -> Result<()> {
    let cwd = env::current_dir()?;
    let lock_path = cwd.join("coldbrew.lock");
    let config_path = cwd.join("coldbrew.toml");

    // Check if lockfile exists
    if !lock_path.exists() {
        return Err(ColdbrewError::LockfileNotFound);
    }

    // Load lockfile
    let lockfile = Lockfile::load(&lock_path)?;

    // Check if lockfile is in sync with config (if config exists)
    if config_path.exists() {
        let config = ProjectConfig::load(&config_path)?;
        if !lockfile.is_in_sync(&config) {
            return Err(ColdbrewError::LockfileOutOfSync);
        }
    }

    output.info(&format!(
        "Installing {} packages from lockfile...",
        lockfile.packages.len()
    ));

    let installed = ops::install::install_from_lockfile(paths, &lockfile, force, output).await?;

    output.success(&format!(
        "Installed {} packages from lockfile",
        installed.len()
    ));

    // Show path hint for last package with binaries
    if let Some(pkg) = installed.iter().find(|p| p.has_binaries() && !p.keg_only) {
        hint_path(paths, pkg, output);
    }

    Ok(())
}

fn hint_path(paths: &Paths, installed: &crate::core::package::InstalledPackage, output: &Output) {
    if installed.keg_only || !installed.has_binaries() {
        return;
    }

    let bin_dir = paths.bin_dir();
    let path_var = env::var("PATH").unwrap_or_default();
    if !path_includes_dir(&path_var, &bin_dir) {
        output.hint(&format!(
            "Add {} to your PATH to use installed binaries",
            bin_dir.display()
        ));
    }
}

fn path_includes_dir(path_var: &str, dir: &Path) -> bool {
    let normalized_dir = normalize_path(dir);
    env::split_paths(path_var).any(|entry| normalize_path(&entry) == normalized_dir)
}

fn normalize_path(path: &Path) -> PathBuf {
    path.components().collect()
}
