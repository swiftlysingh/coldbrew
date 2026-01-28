//! Upgrade command - upgrade installed packages

use crate::cli::output::Output;
use crate::error::Result;
use crate::ops;
use crate::storage::Paths;

/// Execute the upgrade command
pub async fn execute(packages: &[String], yes: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;

    output.info("Checking for upgrades...");

    let upgrades = ops::upgrade::check_upgrades(&paths, packages).await?;

    if upgrades.is_empty() {
        output.success("All packages are up to date");
        return Ok(());
    }

    // Show available upgrades
    output.section("Available upgrades");
    for upgrade in &upgrades {
        println!(
            "  {} {} -> {}",
            Output::package_name(&upgrade.name),
            console::style(&upgrade.current_version).dim(),
            Output::version(&upgrade.new_version)
        );
    }
    println!();

    // Confirm or auto-accept
    let proceed = if yes {
        true
    } else {
        dialoguer::Confirm::new()
            .with_prompt("Proceed with upgrade?")
            .default(true)
            .interact()?
    };

    if !proceed {
        output.info("Upgrade cancelled");
        return Ok(());
    }

    // Perform upgrades
    for upgrade in &upgrades {
        output.info(&format!(
            "Upgrading {} to {}",
            Output::package_name(&upgrade.name),
            Output::version(&upgrade.new_version)
        ));

        let result = ops::upgrade::upgrade_package(&paths, upgrade, output).await;

        match result {
            Ok(_) => {
                output.success(&format!(
                    "Upgraded {} to {}",
                    Output::package_name(&upgrade.name),
                    Output::version(&upgrade.new_version)
                ));
            }
            Err(e) => {
                output.error(&format!("Failed to upgrade {}: {}", upgrade.name, e));
            }
        }
    }

    Ok(())
}
