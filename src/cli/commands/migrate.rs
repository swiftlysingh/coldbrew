//! Migrate command - import Homebrew-installed formulas

use crate::cli::output::Output;
use crate::error::Result;
use crate::ops;
use crate::ops::migrate::{MigratedFormula, MigrationFailure, MigrationSkip};
use crate::storage::Paths;
use dialoguer::Confirm;
use std::io::IsTerminal;

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

    if !summary.dry_run && !summary.migrated.is_empty() {
        if can_prompt() {
            output.section("Homebrew cleanup");
            output.info(&format!(
                "Migrated formulas: {}",
                format_migrated_list(&summary.migrated, 10)
            ));

            let proceed = Confirm::new()
                .with_prompt("Remove these Homebrew installs?")
                .default(false)
                .interact()?;

            if proceed {
                match ops::migrate::cleanup_brew_installs(brew, &summary.migrated, output).await {
                    Ok(cleanup) => {
                        output.success(&format!(
                            "Homebrew cleanup complete: removed {}, {} failed",
                            cleanup.removed.len(),
                            cleanup.failed.len()
                        ));

                        if !cleanup.failed.is_empty() {
                            output.section("Failed removals");
                            for MigrationFailure { name, error } in &cleanup.failed {
                                output.list_item(name, Some(error));
                            }
                        }
                    }
                    Err(err) => {
                        output.warning(&format!("Failed to uninstall Homebrew formulas: {}", err));
                    }
                }
            } else {
                output.info("Skipping Homebrew cleanup");
            }
        } else {
            output.warning("Skipping Homebrew cleanup prompt (non-interactive session).");
            output
                .hint("Re-run `crew migrate` in an interactive shell to remove Homebrew installs.");
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

fn format_migrated_list(items: &[MigratedFormula], limit: usize) -> String {
    let shown: Vec<String> = items
        .iter()
        .take(limit)
        .map(|item| format!("{} {}", item.name, item.version))
        .collect();

    if items.len() <= limit {
        return shown.join(", ");
    }

    format!("{} and {} more", shown.join(", "), items.len() - limit)
}

fn can_prompt() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}
