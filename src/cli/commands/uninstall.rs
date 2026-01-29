//! Uninstall command - remove packages

use crate::cli::commands::spec::resolve_installed_spec;
use crate::cli::output::Output;
use crate::error::Result;
use crate::ops;
use crate::storage::{Cellar, Paths};

/// Execute the uninstall command
pub async fn execute(
    packages: &[String],
    all: bool,
    with_deps: bool,
    output: &Output,
) -> Result<()> {
    let paths = Paths::new()?;
    let cellar = Cellar::new(paths.clone());

    for package in packages {
        let (name, version) = resolve_installed_spec(package, &cellar)?;

        output.info(&format!("Uninstalling {}", Output::package_name(&name)));

        let result =
            ops::uninstall::uninstall(&paths, &name, version.as_deref(), all, with_deps, output)
                .await;

        match result {
            Ok(removed) => {
                for (name, version) in &removed {
                    output.success(&format!(
                        "Uninstalled {} {}",
                        Output::package_name(name),
                        Output::version(version)
                    ));
                }
            }
            Err(e) => {
                output.error(&format!("Failed to uninstall {}: {}", name, e));
                if let Some(suggestion) = e.suggestion() {
                    output.hint(suggestion);
                }
                return Err(e);
            }
        }
    }

    Ok(())
}
