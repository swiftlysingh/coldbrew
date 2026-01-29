//! Coldbrew CLI entry point

use clap::CommandFactory;
use clap_complete::generate;
use coldbrew::cli::commands;
use coldbrew::cli::output::Output;
use coldbrew::cli::{Cli, Commands};
use coldbrew::error::Result;
use coldbrew::storage::Paths;
use std::io;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    if let Err(e) = run().await {
        let output = Output::new(false, false);
        output.error(&e.to_string());

        if let Some(suggestion) = e.suggestion() {
            output.hint(suggestion);
        }

        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse_args();
    let output = Output::new(cli.quiet, cli.verbose);

    // Initialize paths on first run
    let paths = Paths::new()?;
    let _ = paths.init();

    match cli.command {
        Some(Commands::Update) => {
            commands::update::execute(&output).await?;
        }

        Some(Commands::Search { query, extended }) => {
            commands::search::execute(&query, extended, &output).await?;
        }

        Some(Commands::Info { package, format }) => {
            commands::info::execute(&package, &format, &output).await?;
        }

        Some(Commands::Install {
            packages,
            skip_deps,
            force,
        }) => {
            commands::install::execute(&packages, skip_deps, force, &output).await?;
        }

        Some(Commands::Uninstall {
            packages,
            all,
            with_deps,
        }) => {
            commands::uninstall::execute(&packages, all, with_deps, &output).await?;
        }

        Some(Commands::Upgrade { packages, yes }) => {
            commands::upgrade::execute(&packages, yes, &output).await?;
        }

        Some(Commands::List {
            names_only,
            versions,
        }) => {
            commands::list::execute(names_only, versions.as_deref(), &output).await?;
        }

        Some(Commands::Which { binary }) => {
            commands::which::execute(&binary, &output).await?;
        }

        Some(Commands::Pin { package }) => {
            commands::pin::execute(&package, &output).await?;
        }

        Some(Commands::Unpin { package }) => {
            commands::pin::execute_unpin(&package, &output).await?;
        }

        Some(Commands::Default { package }) => {
            commands::default::execute(&package, &output).await?;
        }

        Some(Commands::Dependents { package }) => {
            commands::dependents::execute(&package, &output).await?;
        }

        Some(Commands::Init { force }) => {
            commands::init::execute(force, &output).await?;
        }

        Some(Commands::Lock) => {
            commands::lock::execute(&output).await?;
        }

        Some(Commands::Tap { tap, remove }) => {
            commands::tap::execute(tap.as_deref(), remove, &output).await?;
        }

        Some(Commands::Space { details }) => {
            commands::space::execute(details, &output).await?;
        }

        Some(Commands::Clean { all, dry_run }) => {
            commands::clean::execute(dry_run, all, &output).await?;
        }

        Some(Commands::Link { package, force }) => {
            commands::link::execute(&package, force, &output).await?;
        }

        Some(Commands::Unlink { package }) => {
            commands::link::execute_unlink(&package, &output).await?;
        }

        Some(Commands::Shell { shell }) => {
            commands::shell::execute(shell.as_deref(), &output).await?;
        }

        Some(Commands::Doctor) => {
            commands::doctor::execute(&output).await?;
        }

        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "crew", &mut io::stdout());
        }

        Some(Commands::Exec {
            package,
            binary,
            args,
        }) => {
            execute_shim(&package, &binary, &args).await?;
        }

        None => {
            // No command - show help
            Cli::command().print_help()?;
        }
    }

    Ok(())
}

/// Execute a binary through the shim mechanism
async fn execute_shim(package: &str, binary: &str, args: &[String]) -> Result<()> {
    use coldbrew::config::{GlobalConfig, VersionFileDetector};
    use coldbrew::storage::{Cellar, ShimManager};
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    let paths = Paths::new()?;
    let cellar = Cellar::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());
    let config = GlobalConfig::load(&paths)?;

    // Get version from various sources
    let cwd = std::env::current_dir()?;
    let version_files = VersionFileDetector::new(cwd.clone());
    let detected = version_files.detect_for_package(package)?;

    // Priority: version file > global default > latest installed
    let version = detected
        .map(|v| v.version)
        .or_else(|| config.get_default(package))
        .or_else(|| cellar.latest_version(package).ok().flatten())
        .ok_or_else(|| coldbrew::ColdbrewError::NoDefaultVersion(package.to_string()))?;

    let binary_path = shim_manager.real_binary_path(package, &version, binary);

    if !binary_path.exists() {
        return Err(coldbrew::ColdbrewError::PackageNotInstalled {
            name: package.to_string(),
            version,
        });
    }

    // Build environment with library paths for dependencies
    let pkg = cellar.get_package(package, &version)?;
    let mut lib_paths = Vec::new();

    for dep in &pkg.runtime_dependencies {
        let lib_dir = dep.path.join("lib");
        if lib_dir.exists() {
            lib_paths.push(lib_dir.to_string_lossy().to_string());
        }
    }

    // Execute the real binary
    let mut cmd = Command::new(&binary_path);
    cmd.args(args);

    if !lib_paths.is_empty() {
        let existing = std::env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
        let new_path = if existing.is_empty() {
            lib_paths.join(":")
        } else {
            format!("{}:{}", lib_paths.join(":"), existing)
        };
        cmd.env("DYLD_LIBRARY_PATH", new_path);
    }

    // Replace this process with the target binary
    let err = cmd.exec();
    Err(coldbrew::ColdbrewError::Other(format!(
        "Failed to exec {}: {}",
        binary_path.display(),
        err
    )))
}
