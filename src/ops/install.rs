//! Package installation

use crate::cli::output::Output;
use crate::core::package::{InstalledPackage, PackageMetadata, RuntimeDependency};
use crate::core::version::{version_matches, Version};
use crate::core::{DependencyResolver, Formula, Platform};
use crate::error::{ColdbrewError, Result};
use crate::ops::verify;
use crate::registry::{GhcrClient, Index};
use crate::storage::{Cache, Cellar, Paths, ShimManager};
use std::collections::HashMap;

struct InstallContext<'a> {
    paths: &'a Paths,
    cache: &'a Cache,
    shim_manager: &'a ShimManager,
    ghcr: &'a GhcrClient,
    platform: &'a Platform,
    cellar: &'a Cellar,
    output: &'a Output,
}

struct InstallOptions<'a> {
    installed_as_dependency: bool,
    installed_for: Option<&'a str>,
    force: bool,
}

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
    let ctx = InstallContext {
        paths,
        cache: &cache,
        shim_manager: &shim_manager,
        ghcr: &ghcr,
        platform: &platform,
        cellar: &cellar,
        output,
    };

    let formulas = index.list_formulas()?;
    let mut formula_map = HashMap::new();
    for formula in formulas {
        formula_map.insert(formula.name.clone(), formula);
    }

    let root_formula = formula_map
        .get(name)
        .ok_or_else(|| ColdbrewError::PackageNotFound(name.to_string()))?;

    let install_order: Vec<String> = if skip_deps {
        vec![name.to_string()]
    } else {
        let mut resolver = DependencyResolver::new();
        resolver.add_formulas(formula_map.values().cloned());
        resolver
            .resolve(name)?
            .into_iter()
            .map(|dep| dep.name)
            .collect()
    };

    if !skip_deps && !root_formula.dependencies.is_empty() {
        output.info(&format!(
            "Installing {} dependencies...",
            root_formula.dependencies.len()
        ));
    }

    let mut installed_versions: HashMap<String, String> = HashMap::new();
    let mut root_installed: Option<InstalledPackage> = None;

    for pkg_name in install_order {
        let formula = formula_map
            .get(&pkg_name)
            .ok_or_else(|| ColdbrewError::PackageNotFound(pkg_name.clone()))?;

        let is_root = pkg_name == name;

        if !is_root {
            if let Some(existing) = cellar.latest_version(&pkg_name)? {
                output.debug(&format!(
                    "Dependency {} already installed at {}",
                    pkg_name, existing
                ));
                installed_versions.insert(pkg_name.clone(), existing);
                continue;
            }
        }

        let target_version = if is_root {
            match version {
                Some(requested) => {
                    resolve_requested_version(&pkg_name, requested, &formula.versions.stable)?
                }
                None => formula.versions.stable.clone(),
            }
        } else {
            formula.versions.stable.clone()
        };

        if is_root && cellar.is_installed(&pkg_name, &target_version) && !force {
            return Err(ColdbrewError::PackageAlreadyInstalled {
                name: pkg_name.clone(),
                version: target_version.clone(),
            });
        }

        let runtime_deps = if is_root && skip_deps {
            Vec::new()
        } else {
            resolve_runtime_deps(formula, &installed_versions, &cellar, paths)?
        };

        let options = InstallOptions {
            installed_as_dependency: !is_root,
            installed_for: if is_root { None } else { Some(name) },
            force: is_root && force,
        };
        let installed = install_single(
            &ctx,
            &pkg_name,
            &target_version,
            formula,
            runtime_deps,
            options,
        )
        .await?;

        installed_versions.insert(pkg_name.clone(), installed.version.clone());

        if is_root {
            root_installed = Some(installed);
        }
    }

    root_installed.ok_or_else(|| ColdbrewError::PackageNotFound(name.to_string()))
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

fn resolve_runtime_deps(
    formula: &Formula,
    installed_versions: &HashMap<String, String>,
    cellar: &Cellar,
    paths: &Paths,
) -> Result<Vec<RuntimeDependency>> {
    let mut runtime_deps = Vec::new();

    for dep_name in &formula.dependencies {
        let version = if let Some(version) = installed_versions.get(dep_name) {
            version.clone()
        } else if let Some(version) = cellar.latest_version(dep_name)? {
            version
        } else {
            return Err(ColdbrewError::DependencyResolutionFailed {
                package: formula.name.clone(),
                dep: dep_name.clone(),
            });
        };

        runtime_deps.push(RuntimeDependency {
            name: dep_name.clone(),
            version: version.clone(),
            path: paths.cellar_package(dep_name, &version),
        });
    }

    Ok(runtime_deps)
}

fn resolve_requested_version(name: &str, requested: &str, stable: &str) -> Result<String> {
    match Version::parse(stable) {
        Ok(stable_version) => {
            if version_matches(&stable_version, requested) {
                Ok(stable.to_string())
            } else {
                Err(ColdbrewError::VersionNotAvailable {
                    name: name.to_string(),
                    requested: requested.to_string(),
                    available: stable.to_string(),
                })
            }
        }
        Err(_) => {
            if stable == requested {
                Ok(stable.to_string())
            } else {
                Err(ColdbrewError::VersionNotAvailable {
                    name: name.to_string(),
                    requested: requested.to_string(),
                    available: stable.to_string(),
                })
            }
        }
    }
}

async fn install_single(
    ctx: &InstallContext<'_>,
    name: &str,
    version: &str,
    formula: &Formula,
    runtime_deps: Vec<RuntimeDependency>,
    options: InstallOptions<'_>,
) -> Result<InstalledPackage> {
    if ctx.cellar.is_installed(name, version) && !options.force {
        return Err(ColdbrewError::PackageAlreadyInstalled {
            name: name.to_string(),
            version: version.to_string(),
        });
    }

    let bottle_tags = ctx.platform.bottle_tags();
    let (bottle_tag, bottle_file) = formula
        .bottle
        .stable
        .as_ref()
        .and_then(|stable| stable.best_for_platform(&bottle_tags))
        .ok_or_else(|| ColdbrewError::NoBottleAvailable {
            package: name.to_string(),
            platform: ctx.platform.bottle_tag(),
        })?;

    ctx.output
        .debug(&format!("Using bottle tag: {}", bottle_tag));

    let bottle_path = if let Some(cached) = ctx.cache.get_cached(name, version, &bottle_tag) {
        ctx.output.debug("Using cached bottle");
        cached
    } else {
        ctx.output.debug("Downloading bottle...");
        let download_path = ctx
            .paths
            .downloads_dir()
            .join(format!("{}-{}.{}.bottle.tar.gz", name, version, bottle_tag));

        std::fs::create_dir_all(ctx.paths.downloads_dir())?;

        let pb = ctx
            .output
            .download_progress(0, &format!("Downloading {}", name));

        ctx.ghcr
            .download_bottle(formula, bottle_file, &download_path, |downloaded, total| {
                if total > 0 {
                    pb.set_length(total);
                }
                pb.set_position(downloaded);
            })
            .await?;

        pb.finish_and_clear();

        download_path
    };

    ctx.output.debug("Verifying checksum...");
    verify::verify_bottle(&bottle_path, &bottle_file.sha256, name)?;

    ctx.output.debug("Extracting to cellar...");
    let install_path = ctx.cellar.install(name, version, &bottle_path)?;

    let mut installed = InstalledPackage::new(
        name.to_string(),
        version.to_string(),
        formula.tap.clone(),
        install_path,
    );
    installed.runtime_dependencies = runtime_deps;
    installed.bottle_tag = Some(bottle_tag);
    installed.bottle_sha256 = Some(bottle_file.sha256.clone());
    installed.keg_only = formula.keg_only;
    installed.caveats = formula.caveats.clone();
    installed.installed_as_dependency = options.installed_as_dependency;
    if let Some(installed_for) = options.installed_for {
        installed.installed_for = Some(installed_for.to_string());
    }

    let binaries = ctx.cellar.get_binaries(name, version)?;
    installed.binaries = binaries.clone();

    if !formula.keg_only && !binaries.is_empty() {
        ctx.output
            .debug(&format!("Creating shims for {} binaries", binaries.len()));
        ctx.shim_manager.create_shims(name, version, &binaries)?;
        installed.linked = true;
    }

    let metadata = PackageMetadata::new(installed.clone(), bottle_file.url.clone());
    ctx.cellar.save_metadata(&metadata)?;

    Ok(installed)
}
