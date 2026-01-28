//! Pin command - prevent package upgrades

use crate::cli::output::Output;
use crate::config::GlobalConfig;
use crate::core::version::parse_package_spec;
use crate::error::{ColdbrewError, Result};
use crate::storage::{Cellar, Paths};

/// Execute the pin command
pub async fn execute(package: &str, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cellar = Cellar::new(paths.clone());
    let (name, version) = parse_package_spec(package);

    // Check if package is installed
    let versions = cellar.get_versions(&name)?;
    if versions.is_empty() {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.clone(),
            version: version.unwrap_or_else(|| "any".to_string()),
        });
    }

    let version_to_pin = version.unwrap_or_else(|| versions.last().unwrap().clone());

    if !versions.contains(&version_to_pin) {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.clone(),
            version: version_to_pin,
        });
    }

    // Update pins file
    let mut config = GlobalConfig::load(&paths)?;
    config.add_pin(&name, &version_to_pin);
    config.save(&paths)?;

    output.success(&format!(
        "Pinned {} at version {}",
        Output::package_name(&name),
        Output::version(&version_to_pin)
    ));
    output.hint("This package will be skipped during 'crew upgrade'");

    Ok(())
}

/// Execute the unpin command
pub async fn execute_unpin(package: &str, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let (name, _) = parse_package_spec(package);

    let mut config = GlobalConfig::load(&paths)?;

    if !config.is_pinned(&name) {
        output.warning(&format!("Package '{}' is not pinned", name));
        return Ok(());
    }

    config.remove_pin(&name);
    config.save(&paths)?;

    output.success(&format!("Unpinned {}", Output::package_name(&name)));

    Ok(())
}
