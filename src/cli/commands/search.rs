//! Search command - find packages

use crate::cli::output::Output;
use crate::error::Result;
use crate::registry::Index;
use crate::storage::Paths;

/// Execute the search command
pub async fn execute(query: &str, extended: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let index = Index::new(paths);

    let results = index.search(query)?;

    if results.is_empty() {
        output.warning(&format!("No packages found matching '{}'", query));
        output.hint("Try 'crew update' to refresh the package index");
        return Ok(());
    }

    output.info(&format!("Found {} packages matching '{}'", results.len(), query));
    println!();

    for formula in results.iter().take(if extended { 50 } else { 20 }) {
        if extended {
            output.package_info(
                &formula.name,
                &formula.versions.stable,
                formula.desc.as_deref(),
            );

            if formula.deprecated {
                println!("  {}", console::style("(deprecated)").red());
            }

            if formula.keg_only {
                println!("  {}", console::style("(keg-only)").yellow());
            }

            println!();
        } else {
            let desc = formula.desc.as_deref().unwrap_or("");
            let desc_truncated = if desc.len() > 60 {
                format!("{}...", &desc[..57])
            } else {
                desc.to_string()
            };

            println!(
                "{} {} - {}",
                console::style(&formula.name).green().bold(),
                console::style(&formula.versions.stable).dim(),
                desc_truncated
            );
        }
    }

    if results.len() > (if extended { 50 } else { 20 }) {
        output.hint(&format!(
            "Showing first {} results. Use --extended to see more.",
            if extended { 50 } else { 20 }
        ));
    }

    Ok(())
}
