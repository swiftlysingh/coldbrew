//! Info command - show package details

use crate::cli::output::Output;
use crate::error::{ColdbrewError, Result};
use crate::registry::Index;
use crate::storage::{Cellar, Paths};

/// Execute the info command
pub async fn execute(package: &str, format: &str, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let index = Index::new(paths.clone());
    let cellar = Cellar::new(paths);

    let formula = index
        .get_formula(package)?
        .ok_or_else(|| ColdbrewError::PackageNotFound(package.to_string()))?;

    if format == "json" {
        let json = serde_json::to_string_pretty(&formula)?;
        println!("{}", json);
        return Ok(());
    }

    // Text format
    println!(
        "{} {}",
        console::style(&formula.name).green().bold(),
        console::style(&formula.versions.stable).cyan()
    );

    if let Some(ref desc) = formula.desc {
        println!("{}", desc);
    }

    if let Some(ref homepage) = formula.homepage {
        println!("{}: {}", console::style("Homepage").bold(), homepage);
    }

    if let Some(ref license) = formula.license {
        println!("{}: {}", console::style("License").bold(), license);
    }

    // Installed versions
    let versions = cellar.get_versions(&formula.name)?;
    if !versions.is_empty() {
        println!(
            "{}: {}",
            console::style("Installed").bold(),
            versions.join(", ")
        );
    }

    // Dependencies
    if !formula.dependencies.is_empty() {
        println!(
            "{}: {}",
            console::style("Dependencies").bold(),
            formula.dependencies.join(", ")
        );
    }

    if !formula.build_dependencies.is_empty() {
        println!(
            "{}: {}",
            console::style("Build dependencies").bold(),
            formula.build_dependencies.join(", ")
        );
    }

    // Bottle availability
    let tags = formula.available_bottle_tags();
    if !tags.is_empty() {
        println!("{}: {}", console::style("Bottles").bold(), tags.join(", "));
    }

    // Flags
    let mut flags = Vec::new();
    if formula.keg_only {
        flags.push("keg-only");
    }
    if formula.deprecated {
        flags.push("deprecated");
    }
    if formula.disabled {
        flags.push("disabled");
    }
    if !flags.is_empty() {
        println!("{}: {}", console::style("Flags").bold(), flags.join(", "));
    }

    // Caveats
    if let Some(ref caveats) = formula.caveats {
        output.caveats(caveats);
    }

    Ok(())
}
