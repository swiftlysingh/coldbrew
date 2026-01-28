//! Package installation

use crate::cli::output::Output;
use crate::core::package::{InstalledPackage, PackageMetadata, RuntimeDependency};
use crate::core::{DependencyResolver, Formula, Platform};
use crate::error::{ColdbrewError, Result};
use crate::ops::verify;
use crate::registry::{GhcrClient, Index};
use crate::storage::{Cache, Cellar, Paths, ShimManager};
use std::path::PathBuf;

/// Install a package
pub async fn install(
    paths: &Paths,
    name: &str,
    version: Option<&str>,
    skip_deps: bool,
    force: bool,
    output: &Output,
) -> Result<InstalledPackage> {
    let index = Index::new(paths.clone());
    let cellar = Cellar::new(paths.clone());
    let cache = Cache::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());
    let ghcr = GhcrClient::new()?;
    let platform = Platform::detect()?;

    // Get formula
    let formula = index
        .get_formula(name)?
        .ok_or_else(|| ColdbrewError::PackageNotFound(name.to_string()))?;

    // Determine version to install
    let target_version = version.unwrap_or(&formula.versions.stable);

    // Check if already installed
    if cellar.is_installed(name, target_version) && !force {
        return Err(ColdbrewError::PackageAlreadyInstalled {
            name: name.to_string(),
            version: target_version.to_string(),
        });
    }

    // Install dependencies first
    let mut runtime_deps = Vec::new();
    if !skip_deps && !formula.dependencies.is_empty() {
        output.info(&format!(
            "Installing {} dependencies...",
            formula.dependencies.len()
        ));

        for dep_name in &formula.dependencies {
            // Check if already installed
            if let Some(version) = cellar.latest_version(dep_name)? {
                output.debug(&format!("Dependency {} already installed at {}", dep_name, version));
                runtime_deps.push(RuntimeDependency {
                    name: dep_name.clone(),
                    version: version.clone(),
                    path: paths.cellar_package(dep_name, &version),
                });
                continue;
            }

            // Install dependency
            output.debug(&format!("Installing dependency: {}", dep_name));
            let dep = install(paths, dep_name, None, false, false, output).await?;
            runtime_deps.push(RuntimeDependency {
                name: dep.name.clone(),
                version: dep.version.clone(),
                path: dep.cellar_path.clone(),
            });
        }
    }

    // Find appropriate bottle
    let bottle_tags = platform.bottle_tags();
    let (bottle_tag, bottle_file) = formula
        .bottle
        .stable
        .as_ref()
        .and_then(|stable| stable.best_for_platform(&bottle_tags))
        .ok_or_else(|| ColdbrewError::NoBottleAvailable {
            package: name.to_string(),
            platform: platform.bottle_tag(),
        })?;

    output.debug(&format!("Using bottle tag: {}", bottle_tag));

    // Check cache
    let bottle_path = if let Some(cached) = cache.get_cached(name, target_version, bottle_tag) {
        output.debug("Using cached bottle");
        cached
    } else {
        // Download bottle
        output.debug("Downloading bottle...");
        let download_path = paths.downloads_dir().join(format!(
            "{}-{}.{}.bottle.tar.gz",
            name, target_version, bottle_tag
        ));

        std::fs::create_dir_all(paths.downloads_dir())?;

        let pb = output.download_progress(0, &format!("Downloading {}", name));

        ghcr.download_bottle(&formula, bottle_file, &download_path, |downloaded, total| {
            if total > 0 {
                pb.set_length(total);
            }
            pb.set_position(downloaded);
        })
        .await?;

        pb.finish_and_clear();

        download_path
    };

    // Verify checksum
    output.debug("Verifying checksum...");
    verify::verify_bottle(&bottle_path, &bottle_file.sha256, name)?;

    // Extract to cellar
    output.debug("Extracting to cellar...");
    let install_path = cellar.install(name, target_version, &bottle_path)?;

    // Create package metadata
    let mut installed = InstalledPackage::new(
        name.to_string(),
        target_version.to_string(),
        formula.tap.clone(),
        install_path,
    );
    installed.runtime_dependencies = runtime_deps;
    installed.bottle_tag = Some(bottle_tag.to_string());
    installed.bottle_sha256 = Some(bottle_file.sha256.clone());
    installed.keg_only = formula.keg_only;
    installed.caveats = formula.caveats.clone();

    // Get binaries
    let binaries = cellar.get_binaries(name, target_version)?;
    installed.binaries = binaries.clone();

    // Create shims (unless keg-only)
    if !formula.keg_only && !binaries.is_empty() {
        output.debug(&format!("Creating shims for {} binaries", binaries.len()));
        shim_manager.create_shims(name, target_version, &binaries)?;
        installed.linked = true;
    }

    // Save metadata
    let metadata = PackageMetadata::new(installed.clone(), bottle_file.url.clone());
    cellar.save_metadata(&metadata)?;

    Ok(installed)
}

/// Install from a lockfile
pub async fn install_from_lockfile(
    paths: &Paths,
    lockfile: &crate::config::Lockfile,
    output: &Output,
) -> Result<Vec<InstalledPackage>> {
    let mut installed = Vec::new();

    for (name, locked) in &lockfile.packages {
        output.info(&format!("Installing {} {}...", name, locked.version));

        let pkg = install(
            paths,
            name,
            Some(&locked.version),
            true, // Skip deps, lockfile has them
            false,
            output,
        )
        .await?;

        installed.push(pkg);
    }

    Ok(installed)
}
