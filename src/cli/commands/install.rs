//! Install command - install packages

use crate::cli::output::Output;
use crate::core::version::parse_package_spec;
use crate::error::Result;
use crate::ops;
use crate::storage::Paths;

/// Execute the install command
pub async fn execute(
    packages: &[String],
    skip_deps: bool,
    force: bool,
    output: &Output,
) -> Result<()> {
    let paths = Paths::new()?;
    paths.init()?;

    for package in packages {
        let (name, version) = parse_package_spec(package);

        output.info(&format!(
            "Installing {}{}",
            Output::package_name(&name),
            version
                .as_ref()
                .map(|v| format!("@{}", v))
                .unwrap_or_default()
        ));

        let result = ops::install::install(
            &paths,
            &name,
            version.as_deref(),
            skip_deps,
            force,
            output,
        )
        .await;

        match result {
            Ok(installed) => {
                output.success(&format!(
                    "Installed {} {}",
                    Output::package_name(&installed.name),
                    Output::version(&installed.version)
                ));

                if let Some(ref caveats) = installed.caveats {
                    output.caveats(caveats);
                }
            }
            Err(e) => {
                output.error(&format!("Failed to install {}: {}", name, e));
                if let Some(suggestion) = e.suggestion() {
                    output.hint(suggestion);
                }
                return Err(e);
            }
        }
    }

    Ok(())
}
