//! Link command - force-link keg-only packages

use crate::cli::output::Output;
use crate::core::version::parse_package_spec;
use crate::error::{ColdbrewError, Result};
use crate::storage::{Cellar, Paths, ShimManager};

/// Execute the link command
pub async fn execute(package: &str, force: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths);

    let (name, version) = parse_package_spec(package);

    // Get the version to link
    let versions = cellar.get_versions(&name)?;
    if versions.is_empty() {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.clone(),
            version: version.unwrap_or_else(|| "any".to_string()),
        });
    }

    let version_to_link = version.unwrap_or_else(|| versions.last().unwrap().clone());

    if !versions.contains(&version_to_link) {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.clone(),
            version: version_to_link,
        });
    }

    // Get binaries
    let binaries = cellar.get_binaries(&name, &version_to_link)?;

    if binaries.is_empty() {
        output.warning(&format!(
            "{} {} has no binaries to link",
            name, version_to_link
        ));
        return Ok(());
    }

    // Check for existing shims
    for binary in &binaries {
        if shim_manager.has_shim(binary) && !force {
            output.warning(&format!(
                "Shim for '{}' already exists. Use --force to overwrite",
                binary
            ));
            return Ok(());
        }
    }

    // Create shims
    let created = shim_manager.create_shims(&name, &version_to_link, &binaries)?;

    output.success(&format!(
        "Linked {} {} ({} binaries)",
        Output::package_name(&name),
        Output::version(&version_to_link),
        created.len()
    ));

    for binary in &binaries {
        println!("  {}", binary);
    }

    Ok(())
}

/// Execute the unlink command
pub async fn execute_unlink(package: &str, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths);

    let (name, version) = parse_package_spec(package);

    // Get binaries to unlink
    let versions = cellar.get_versions(&name)?;
    if versions.is_empty() {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.clone(),
            version: version.unwrap_or_else(|| "any".to_string()),
        });
    }

    let version_to_check = version.unwrap_or_else(|| versions.last().unwrap().clone());
    let binaries = cellar.get_binaries(&name, &version_to_check)?;

    if binaries.is_empty() {
        output.info(&format!("{} has no binaries to unlink", name));
        return Ok(());
    }

    shim_manager.remove_shims(&binaries)?;

    output.success(&format!(
        "Unlinked {} ({} binaries)",
        Output::package_name(&name),
        binaries.len()
    ));

    Ok(())
}
