//! Which command - show which package provides a binary

use crate::cli::output::Output;
use crate::error::Result;
use crate::storage::{Cellar, Paths, ShimManager};

/// Execute the which command
pub async fn execute(binary: &str, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());

    // First check if there's a shim
    if shim_manager.has_shim(binary) {
        let shims = shim_manager.list_shims()?;
        if let Some(shim) = shims.iter().find(|s| s.name == binary) {
            output.info(&format!(
                "'{}' is provided by {}",
                binary,
                Output::package_name(&shim.package)
            ));
            println!("  Shim: {}", shim.path.display());

            // Show available versions
            let versions = cellar.get_versions(&shim.package)?;
            if !versions.is_empty() {
                println!("  Versions: {}", versions.join(", "));
            }

            return Ok(());
        }
    }

    // Search through installed packages
    let packages = cellar.list_packages()?;
    for pkg in packages {
        let binaries = cellar.get_binaries(&pkg.name, &pkg.version)?;
        if binaries.contains(&binary.to_string()) {
            output.info(&format!(
                "'{}' is provided by {} {}",
                binary,
                Output::package_name(&pkg.name),
                Output::version(&pkg.version)
            ));
            println!(
                "  Binary: {}",
                pkg.cellar_path.join("bin").join(binary).display()
            );
            return Ok(());
        }
    }

    output.warning(&format!("Binary '{}' not found in any installed package", binary));
    output.hint("Use 'crew search <name>' to find packages");

    Ok(())
}
