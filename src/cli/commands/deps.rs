//! Deps command - show package dependencies

use crate::cli::output::Output;
use crate::core::DependencyResolver;
use crate::error::{ColdbrewError, Result};
use crate::registry::Index;
use crate::storage::Paths;

/// Execute the deps command
pub async fn execute(package: &str, tree: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let index = Index::new(paths);

    let formula = index
        .get_formula(package)?
        .ok_or_else(|| ColdbrewError::PackageNotFound(package.to_string()))?;

    if formula.dependencies.is_empty() {
        output.info(&format!(
            "{} has no dependencies",
            Output::package_name(package)
        ));
        return Ok(());
    }

    if tree {
        // Build and display dependency tree
        let all_formulas = index.list_formulas()?;
        let mut resolver = DependencyResolver::new();
        resolver.add_formulas(all_formulas);

        match resolver.dependency_tree(package) {
            Ok(dep_tree) => {
                output.info(&format!(
                    "Dependency tree for {} ({} total dependencies)",
                    Output::package_name(package),
                    dep_tree.total_count()
                ));
                println!();
                print!("{}", dep_tree.pretty_print());
            }
            Err(e) => {
                output.warning(&format!("Could not build full tree: {}", e));
                // Fall back to simple list
                output.info("Direct dependencies:");
                for dep in &formula.dependencies {
                    println!("  {}", dep);
                }
            }
        }
    } else {
        output.info(&format!(
            "Dependencies for {}:",
            Output::package_name(package)
        ));
        for dep in &formula.dependencies {
            println!("  {}", dep);
        }

        if !formula.build_dependencies.is_empty() {
            println!();
            output.info("Build dependencies:");
            for dep in &formula.build_dependencies {
                println!("  {}", dep);
            }
        }
    }

    Ok(())
}
