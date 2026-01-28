//! Package upgrade operations

use crate::cli::output::Output;
use crate::config::GlobalConfig;
use crate::core::version::Version;
use crate::error::Result;
use crate::ops::install;
use crate::registry::Index;
use crate::storage::{Cellar, Paths};

/// Information about an available upgrade
#[derive(Debug, Clone)]
pub struct UpgradeInfo {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub is_major: bool,
}

/// Check for available upgrades
pub async fn check_upgrades(paths: &Paths, filter: &[String]) -> Result<Vec<UpgradeInfo>> {
    let cellar = Cellar::new(paths.clone());
    let index = Index::new(paths.clone());
    let config = GlobalConfig::load(paths)?;

    let installed = cellar.list_packages()?;
    let mut upgrades = Vec::new();

    for pkg in installed {
        // Skip if not in filter (if filter provided)
        if !filter.is_empty() && !filter.contains(&pkg.name) {
            continue;
        }

        // Skip if pinned
        if config.is_pinned(&pkg.name) {
            continue;
        }

        // Get latest version from index
        if let Ok(Some(formula)) = index.get_formula(&pkg.name) {
            let latest = &formula.versions.stable;

            if latest != &pkg.version {
                // Check if this is a major upgrade
                let current = Version::parse(&pkg.version).ok();
                let new = Version::parse(latest).ok();

                let is_major = match (current, new) {
                    (Some(c), Some(n)) => c.major() != n.major(),
                    _ => false,
                };

                upgrades.push(UpgradeInfo {
                    name: pkg.name.clone(),
                    current_version: pkg.version.clone(),
                    new_version: latest.clone(),
                    is_major,
                });
            }
        }
    }

    // Sort by name
    upgrades.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(upgrades)
}

/// Upgrade a single package
pub async fn upgrade_package(
    paths: &Paths,
    upgrade: &UpgradeInfo,
    output: &Output,
) -> Result<()> {
    // Install new version
    install::install(
        paths,
        &upgrade.name,
        Some(&upgrade.new_version),
        false,
        true, // Force to allow upgrade
        output,
    )
    .await?;

    // Update default to new version
    let mut config = GlobalConfig::load(paths)?;
    config.set_default(&upgrade.name, &upgrade.new_version);
    config.save(paths)?;

    Ok(())
}

/// Upgrade all packages
pub async fn upgrade_all(
    paths: &Paths,
    yes: bool,
    output: &Output,
) -> Result<Vec<UpgradeInfo>> {
    let upgrades = check_upgrades(paths, &[]).await?;

    if upgrades.is_empty() {
        return Ok(upgrades);
    }

    for upgrade in &upgrades {
        output.info(&format!(
            "Upgrading {} {} -> {}",
            upgrade.name, upgrade.current_version, upgrade.new_version
        ));

        if let Err(e) = upgrade_package(paths, upgrade, output).await {
            output.error(&format!("Failed to upgrade {}: {}", upgrade.name, e));
        }
    }

    Ok(upgrades)
}
