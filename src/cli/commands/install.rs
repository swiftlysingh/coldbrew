//! Install command - install packages

use crate::cli::output::Output;
use crate::core::version::parse_package_spec;
use crate::error::{ColdbrewError, Result};
use crate::ops;
use crate::storage::{Cellar, Paths};
use std::env;
use std::path::{Path, PathBuf};

/// Execute the install command
pub async fn execute(
    packages: &[String],
    skip_deps: bool,
    force: bool,
    output: &Output,
) -> Result<()> {
    let paths = Paths::new()?;
    paths.init()?;

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
