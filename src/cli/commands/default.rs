//! Default command - set or show default package version

use crate::cli::output::Output;
use crate::config::GlobalConfig;
use crate::core::version::parse_package_spec;
use crate::error::{ColdbrewError, Result};
use crate::storage::{Cellar, Paths};

/// Execute the default command
pub async fn execute(package: &str, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cellar = Cellar::new(paths.clone());
    let (name, version) = parse_package_spec(package);

    // Get installed versions
    let versions = cellar.get_versions(&name)?;
    if versions.is_empty() {
        return Err(ColdbrewError::PackageNotInstalled {
            name: name.clone(),
            version: version.unwrap_or_else(|| "any".to_string()),
        });
    }

    if let Some(version) = version {
        // Set default version
        if !versions.contains(&version) {
            return Err(ColdbrewError::PackageNotInstalled {
                name: name.clone(),
                version,
            });
        }

        let mut config = GlobalConfig::load(&paths)?;
        config.set_default(&name, &version);
        config.save(&paths)?;

        output.success(&format!(
            "Set default {} to version {}",
            Output::package_name(&name),
            Output::version(&version)
        ));
    } else {
        // Show current default
        let config = GlobalConfig::load(&paths)?;
        let default = config.get_default(&name);

        output.info(&format!("Versions of {}:", Output::package_name(&name)));
        for v in &versions {
            let is_default = default.as_ref() == Some(v);
            if is_default {
                println!("  {} {}", Output::version(v), console::style("(default)").green());
            } else {
                println!("  {}", Output::version(v));
            }
        }

        if default.is_none() {
            output.hint(&format!(
                "Set a default with 'coldbrew default {}@<version>'",
                name
            ));
        }
    }

    Ok(())
}
