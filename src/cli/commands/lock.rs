//! Lock command - generate lockfile

use crate::cli::output::Output;
use crate::config::{Lockfile, ProjectConfig};
use crate::error::{ColdbrewError, Result};
use crate::registry::Index;
use crate::storage::Paths;
use std::env;

/// Execute the lock command
pub async fn execute(output: &Output) -> Result<()> {
    let cwd = env::current_dir()?;
    let config_path = cwd.join("coldbrew.toml");

    if !config_path.exists() {
        return Err(ColdbrewError::ProjectNotFound);
    }

    let config = ProjectConfig::load(&config_path)?;
    let paths = Paths::new()?;
    let index = Index::new(paths);

    output.info("Resolving dependencies...");

    let lockfile = Lockfile::generate(&config, &index).await?;
    let lock_path = cwd.join("coldbrew.lock");
    lockfile.save(&lock_path)?;

    output.success(&format!(
        "Generated {} ({} packages)",
        lock_path.display(),
        lockfile.packages.len()
    ));

    Ok(())
}
