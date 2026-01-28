//! Dependents command - show packages that depend on a package

use crate::cli::output::Output;
use crate::core::DependencyResolver;
use crate::error::{ColdbrewError, Result};
use crate::registry::Index;
use crate::storage::{Cellar, Paths};

/// Execute the dependents command
pub async fn execute(package: &str, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let index = Index::new(paths.clone());
    let cellar = Cellar::new(paths);

    // Verify package exists
    let formula = index
        .get_formula(package)?
        .ok_or_else(|| ColdbrewError::PackageNotFound(package.to_string()))?;

    // Build dependency resolver with all formulas
    let all_formulas = index.list_formulas()?;
    let mut resolver = DependencyResolver::new();
    resolver.add_formulas(all_formulas);

    // Find dependents
    let dependents = resolver.get_dependents(package);

    if dependents.is_empty() {
        output.info(&format!(
            "No packages depend on {}",
            Output::package_name(package)
        ));
        return Ok(());
    }

    // Check which are installed
    let installed_packages = cellar.list_packages()?;
    let installed_names: std::collections::HashSet<_> =
        installed_packages.iter().map(|p| p.name.clone()).collect();

    output.info(&format!(
        "Packages that depend on {}:",
        Output::package_name(package)
    ));

    let mut installed_dependents = Vec::new();
    let mut not_installed_dependents = Vec::new();

    for dep in &dependents {
        if installed_names.contains(dep) {
            installed_dependents.push(dep);
        } else {
            not_installed_dependents.push(dep);
        }
    }

    if !installed_dependents.is_empty() {
        println!();
        println!("  {} Installed:", console::style("●").green());
        for dep in installed_dependents {
            println!("    {}", Output::package_name(dep));
        }
    }

    if !not_installed_dependents.is_empty() {
        println!();
        println!("  {} Not installed:", console::style("○").dim());
        for dep in not_installed_dependents.iter().take(10) {
            println!("    {}", dep);
        }
        if not_installed_dependents.len() > 10 {
            println!(
                "    {} more...",
                console::style(format!("({})", not_installed_dependents.len() - 10)).dim()
            );
        }
    }

    Ok(())
}
