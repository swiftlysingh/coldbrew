//! Init command - create coldbrew.toml

use crate::cli::output::Output;
use crate::config::ProjectConfig;
use crate::error::Result;
use std::env;

/// Execute the init command
pub async fn execute(force: bool, output: &Output) -> Result<()> {
    let cwd = env::current_dir()?;
    let config_path = cwd.join("coldbrew.toml");

    if config_path.exists() && !force {
        output.warning("coldbrew.toml already exists in this directory");
        output.hint("Use --force to overwrite");
        return Ok(());
    }

    let config = ProjectConfig::default();
    config.save(&config_path)?;

    output.success(&format!("Created {}", config_path.display()));
    output.hint("Edit coldbrew.toml to add packages, then run 'crew lock'");

    Ok(())
}
