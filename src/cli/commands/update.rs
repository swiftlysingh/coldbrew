//! Update command - fetch latest package index

use crate::cli::output::Output;
use crate::error::Result;
use crate::registry::Index;
use crate::storage::Paths;

/// Execute the update command
pub async fn execute(output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    paths.init()?;

    output.info("Updating package index from Homebrew...");

    let spinner = output.spinner("Downloading formula.json");

    let mut index = Index::new(paths);
    let count = index.update().await?;

    spinner.finish_and_clear();
    output.success(&format!(
        "Updated package index ({} formulas)",
        count
    ));

    Ok(())
}
