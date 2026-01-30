//! Migrate command - import Homebrew-installed formulas

use crate::cli::output::Output;
use crate::error::Result;
use crate::ops;
use crate::ops::migrate::{MigrationFailure, MigrationSkip};
use crate::storage::Paths;

/// Execute the migrate command
pub async fn execute(brew: Option<&str>, dry_run: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    paths.init()?;

    let summary = ops::migrate::migrate(&paths, brew, dry_run, output).await?;

    if !summary.casks.is_empty() {
        output.warning(&format!(
            "Skipping {} Homebrew cask{} (not supported): {}",
            summary.casks.len(),
            if summary.casks.len() == 1 { "" } else { "s" },
            format_list(&summary.casks, 10)
        ));
    }

    for warning in &summary.warnings {
        output.warning(warning);
    }

    if summary.requested == 0 {
        output.info("No user-requested Homebrew formulas to migrate");
        return Ok(());
    }

    let action = if summary.dry_run {
        "would migrate"
    } else {
        "migrated"
    };

    output.success(&format!(
        "Migration complete: {} {}, {} skipped, {} failed",
        action,
        summary.migrated.len(),
        summary.skipped.len(),
        summary.failed.len()
    ));

    if !summary.skipped.is_empty() {
        output.section("Skipped formulas");
        for MigrationSkip { name, reason } in &summary.skipped {
            output.list_item(name, Some(reason));
        }
    }

    if !summary.failed.is_empty() {
        output.section("Failed formulas");
        for MigrationFailure { name, error } in &summary.failed {
            output.list_item(name, Some(error));
        }
    }

    Ok(())
}

fn format_list(items: &[String], limit: usize) -> String {
    if items.len() <= limit {
        return items.join(", ");
    }

    let shown = items[..limit].join(", ");
    format!("{} and {} more", shown, items.len() - limit)
}
