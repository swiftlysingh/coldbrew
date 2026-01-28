//! Tap command - manage third-party repositories

use crate::cli::output::Output;
use crate::error::Result;
use crate::registry::TapManager;
use crate::storage::Paths;

/// Execute the tap command
pub async fn execute(tap: Option<&str>, remove: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let mut tap_manager = TapManager::new(paths);

    match (tap, remove) {
        (Some(tap_name), true) => {
            // Remove tap
            output.info(&format!("Removing tap '{}'", tap_name));
            tap_manager.remove(tap_name)?;
            output.success(&format!("Removed tap '{}'", tap_name));
        }
        (Some(tap_name), false) => {
            // Add tap
            output.info(&format!("Adding tap '{}'", tap_name));
            let spinner = output.spinner(&format!("Cloning {}", tap_name));
            tap_manager.add(tap_name).await?;
            spinner.finish_and_clear();
            output.success(&format!("Added tap '{}'", tap_name));
        }
        (None, _) => {
            // List taps
            let taps = tap_manager.list()?;

            if taps.is_empty() {
                output.info("No taps installed");
                output.hint("Add a tap with 'crew tap user/repo'");
            } else {
                output.info(&format!("{} taps installed:", taps.len()));
                for tap in taps {
                    println!("  {}", tap.full_name());
                }
            }
        }
    }

    Ok(())
}
