//! List command - show installed packages

use crate::cli::output::Output;
use crate::error::Result;
use crate::storage::{Cellar, Paths};

/// Execute the list command
pub async fn execute(names_only: bool, versions: Option<&str>, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cellar = Cellar::new(paths);

    if let Some(package) = versions {
        // Show versions for a specific package
        let versions = cellar.get_versions(package)?;

        if versions.is_empty() {
            output.warning(&format!("Package '{}' is not installed", package));
            return Ok(());
        }

        output.info(&format!(
            "Installed versions of {}:",
            Output::package_name(package)
        ));
        for version in versions {
            println!("  {}", Output::version(&version));
        }
    } else {
        // List all packages
        let packages = cellar.list_packages()?;

        if packages.is_empty() {
            output.info("No packages installed");
            output.hint("Use 'crew install <package>' to install packages");
            return Ok(());
        }

        if names_only {
            for pkg in &packages {
                println!("{}", pkg.name);
            }
        } else {
            output.info(&format!("{} packages installed", packages.len()));
            println!();

            for pkg in &packages {
                let mut flags = Vec::new();
                if pkg.pinned {
                    flags.push("pinned");
                }
                if pkg.keg_only {
                    flags.push("keg-only");
                }

                let flags_str = if flags.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", flags.join(", "))
                };

                println!(
                    "  {} {}{}",
                    Output::package_name(&pkg.name),
                    Output::version(&pkg.version),
                    console::style(flags_str).dim()
                );
            }
        }
    }

    Ok(())
}
