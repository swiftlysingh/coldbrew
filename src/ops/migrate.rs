//! Homebrew migration operations

use crate::cli::output::Output;
use crate::core::package::{InstalledPackage, PackageMetadata, RuntimeDependency};
use crate::core::platform::{Os, Platform};
use crate::error::{ColdbrewError, Result};
use crate::ops;
use crate::ops::relocate;
use crate::registry::Index;
use crate::storage::{Cellar, Paths, ShimManager};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct MigrationSkip {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct MigrationFailure {
    pub name: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct MigratedFormula {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct MigrationSummary {
    pub requested: usize,
    pub migrated: Vec<MigratedFormula>,
    pub skipped: Vec<MigrationSkip>,
    pub failed: Vec<MigrationFailure>,
    pub casks: Vec<String>,
    pub warnings: Vec<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct MigrationCleanupSummary {
    pub removed: Vec<String>,
    pub failed: Vec<MigrationFailure>,
}

impl MigrationSummary {
    fn new(requested: usize, dry_run: bool) -> Self {
        Self {
            requested,
            migrated: Vec::new(),
            skipped: Vec::new(),
            failed: Vec::new(),
            casks: Vec::new(),
            warnings: Vec::new(),
            dry_run,
        }
    }
}

impl MigrationCleanupSummary {
    fn new() -> Self {
        Self {
            removed: Vec::new(),
            failed: Vec::new(),
        }
    }
}

/// Migrate Homebrew-installed formulas into Coldbrew.
pub async fn migrate(
    paths: &Paths,
    brew_override: Option<&str>,
    dry_run: bool,
    output: &Output,
) -> Result<MigrationSummary> {
    let brew = detect_brew(brew_override)?;

    output.info("Reading Homebrew leaves (user-requested formulas)...");
    let leaves_output = run_brew(&brew, &["leaves", "--installed-on-request"]).await?;
    let leaves = parse_brew_leaves(&leaves_output);

    output.info("Reading Homebrew installed formula versions...");
    let versions_output = run_brew(&brew, &["list", "--formula", "--versions"]).await?;
    let versions = parse_brew_formula_versions(&versions_output);

    let mut summary = MigrationSummary::new(leaves.len(), dry_run);

    match run_brew(&brew, &["list", "--cask"]).await {
        Ok(cask_output) => {
            summary.casks = parse_brew_casks(&cask_output);
        }
        Err(e) => {
            summary
                .warnings
                .push(format!("Failed to list Homebrew casks: {}", e));
        }
    }

    if leaves.is_empty() {
        return Ok(summary);
    }

    let index = Index::new(paths.clone());
    let formulas = index.list_formulas()?;
    let mut formula_map = HashMap::new();
    for formula in formulas {
        formula_map.insert(formula.name.clone(), formula);
    }

    let cellar = Cellar::new(paths.clone());
    let mut brew_prefix: Option<PathBuf> = None;
    let mut brew_cellar: Option<PathBuf> = None;

    for name in leaves {
        let Some(formula) = formula_map.get(&name) else {
            summary.skipped.push(MigrationSkip {
                name: name.clone(),
                reason: "Not in Homebrew core index".to_string(),
            });
            continue;
        };

        let Some(installed_version) = versions.get(&name) else {
            summary.skipped.push(MigrationSkip {
                name: name.clone(),
                reason: "Homebrew did not report an installed version".to_string(),
            });
            continue;
        };

        let available_version = formula.version_with_revision();

        if cellar.is_installed(&name, installed_version) {
            summary.skipped.push(MigrationSkip {
                name: name.clone(),
                reason: "Already installed in Coldbrew".to_string(),
            });
            continue;
        }

        if dry_run {
            output.info(&format!(
                "Would migrate {} {}{}",
                Output::package_name(&name),
                Output::version(installed_version),
                if available_version == *installed_version {
                    ""
                } else {
                    " (importing Homebrew keg)"
                }
            ));
            summary.migrated.push(MigratedFormula {
                name: name.clone(),
                version: installed_version.to_string(),
            });
            continue;
        }

        if available_version == *installed_version {
            output.info(&format!(
                "Migrating {} {}",
                Output::package_name(&name),
                Output::version(installed_version)
            ));

            match ops::install::install(
                paths,
                &name,
                Some(installed_version),
                false,
                false,
                output,
            )
            .await
            {
                Ok(_) => {
                    summary.migrated.push(MigratedFormula {
                        name: name.clone(),
                        version: installed_version.to_string(),
                    });
                }
                Err(e) => {
                    summary.failed.push(MigrationFailure {
                        name: name.clone(),
                        error: e.to_string(),
                    });
                }
            }
            continue;
        }

        output.info(&format!(
            "Migrating {} {} (importing Homebrew keg)",
            Output::package_name(&name),
            Output::version(installed_version)
        ));

        let prefix = match brew_prefix.clone() {
            Some(prefix) => prefix,
            None => match brew_path(&brew, &["--prefix"], "prefix").await {
                Ok(prefix) => {
                    brew_prefix = Some(prefix.clone());
                    prefix
                }
                Err(e) => {
                    summary.failed.push(MigrationFailure {
                        name: name.clone(),
                        error: e.to_string(),
                    });
                    continue;
                }
            },
        };

        let cellar_path = match brew_cellar.clone() {
            Some(cellar_path) => cellar_path,
            None => match brew_path(&brew, &["--cellar"], "cellar").await {
                Ok(cellar_path) => {
                    brew_cellar = Some(cellar_path.clone());
                    cellar_path
                }
                Err(e) => {
                    summary.failed.push(MigrationFailure {
                        name: name.clone(),
                        error: e.to_string(),
                    });
                    continue;
                }
            },
        };

        let receipt_deps =
            match read_keg_receipt_dependencies(&cellar_path, &name, installed_version) {
                Ok(Some(deps)) => Some(deps),
                Ok(None) => None,
                Err(e) => {
                    summary.warnings.push(format!(
                        "Failed to read Homebrew receipt for {} {}: {}",
                        name, installed_version, e
                    ));
                    None
                }
            };

        let deps_for_install = receipt_deps
            .as_ref()
            .unwrap_or(&formula.dependencies);

        let installed_versions = match ops::install::install_dependencies_for_root_with_list(
            paths,
            &name,
            Some(deps_for_install),
            output,
        )
        .await
        {
            Ok(versions) => versions,
            Err(e) => {
                summary.failed.push(MigrationFailure {
                    name: name.clone(),
                    error: e.to_string(),
                });
                continue;
            }
        };

        let runtime_deps = match build_runtime_deps(
            &name,
            deps_for_install,
            &installed_versions,
            &cellar,
            paths,
        ) {
            Ok(deps) => deps,
            Err(e) => {
                summary.failed.push(MigrationFailure {
                    name: name.clone(),
                    error: e.to_string(),
                });
                continue;
            }
        };

        let import = KegImport {
            paths,
            formula,
            name: &name,
            version: installed_version,
            brew_cellar: &cellar_path,
            brew_prefix: &prefix,
            runtime_deps,
            output,
        };

        match import_homebrew_keg(import) {
            Ok(_) => {
                summary.migrated.push(MigratedFormula {
                    name: name.clone(),
                    version: installed_version.to_string(),
                });
            }
            Err(e) => {
                summary.failed.push(MigrationFailure {
                    name: name.clone(),
                    error: e.to_string(),
                });
            }
        }
    }

    Ok(summary)
}

async fn brew_path(brew: &Path, args: &[&str], label: &str) -> Result<PathBuf> {
    let output = run_brew(brew, args).await?;
    let path = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| {
            ColdbrewError::Other(format!("Homebrew returned empty {} path", label))
        })?;

    Ok(PathBuf::from(path))
}

fn build_runtime_deps(
    package: &str,
    deps: &[String],
    installed_versions: &HashMap<String, String>,
    cellar: &Cellar,
    paths: &Paths,
) -> Result<Vec<RuntimeDependency>> {
    let mut runtime_deps = Vec::new();

    for dep_name in deps {
        let version = if let Some(version) = installed_versions.get(dep_name) {
            version.clone()
        } else if let Some(version) = cellar.latest_version(dep_name)? {
            version
        } else {
            return Err(ColdbrewError::DependencyResolutionFailed {
                package: package.to_string(),
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

fn read_keg_receipt_dependencies(
    brew_cellar: &Path,
    name: &str,
    version: &str,
) -> Result<Option<Vec<String>>> {
    let receipt_path = brew_cellar
        .join(name)
        .join(version)
        .join("INSTALL_RECEIPT.json");
    if !receipt_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&receipt_path)?;
    let receipt: serde_json::Value = serde_json::from_str(&content)?;

    let mut deps = Vec::new();

    if let Some(runtime) = receipt.get("runtime_dependencies").and_then(|v| v.as_array()) {
        for dep in runtime {
            if let Some(dep_name) = dep
                .get("full_name")
                .or_else(|| dep.get("name"))
                .and_then(|v| v.as_str())
            {
                deps.push(normalize_dependency_name(dep_name));
            }
        }
    }

    if deps.is_empty() {
        if let Some(list) = receipt.get("dependencies").and_then(|v| v.as_array()) {
            for dep in list {
                if let Some(dep_name) = dep.as_str() {
                    deps.push(normalize_dependency_name(dep_name));
                }
            }
        }
    }

    if deps.is_empty() {
        Ok(None)
    } else {
        Ok(Some(deps))
    }
}

fn normalize_dependency_name(name: &str) -> String {
    name.rsplit('/').next().unwrap_or(name).to_string()
}

struct KegImport<'a> {
    paths: &'a Paths,
    formula: &'a crate::core::Formula,
    name: &'a str,
    version: &'a str,
    brew_cellar: &'a Path,
    brew_prefix: &'a Path,
    runtime_deps: Vec<RuntimeDependency>,
    output: &'a Output,
}

fn import_homebrew_keg(import: KegImport<'_>) -> Result<()> {
    let cellar = Cellar::new(import.paths.clone());
    let shim_manager = ShimManager::new(import.paths.clone());
    let platform = Platform::detect()?;

    let source_keg = import.brew_cellar.join(import.name).join(import.version);
    if !source_keg.exists() {
        return Err(ColdbrewError::PathNotFound(source_keg));
    }

    let target_keg = import.paths.cellar_package(import.name, import.version);
    copy_keg_dir(&source_keg, &target_keg)?;
    let result = (|| -> Result<()> {
        if platform.os == Os::MacOS {
            let summary = relocate::relocate_keg(
                &target_keg,
                import.brew_cellar,
                import.brew_prefix,
                import.paths,
                &platform,
                import.output,
            )?;
            if summary.relocated_files > 0 {
                relocate::codesign_macho_tree(&target_keg, &platform, import.output)?;
            }
        }

        let mut installed = InstalledPackage::new(
            import.name.to_string(),
            import.version.to_string(),
            import.formula.tap.clone(),
            target_keg.clone(),
        );
        installed.runtime_dependencies = import.runtime_deps;
        installed.keg_only = import.formula.keg_only;
        installed.caveats = import.formula.caveats.clone();

        let binaries = cellar.get_binaries(import.name, import.version)?;
        installed.binaries = binaries.clone();

        if !import.formula.keg_only && !binaries.is_empty() {
            shim_manager.create_shims(import.name, import.version, &binaries)?;
            installed.linked = true;
        }

        let metadata = PackageMetadata::new(installed, "homebrew keg".to_string());
        cellar.save_metadata(&metadata)?;

        Ok(())
    })();

    if let Err(err) = result {
        if let Err(cleanup_err) = cellar.uninstall(import.name, import.version) {
            import.output.debug(&format!(
                "Failed to cleanup imported keg {} {}: {}",
                import.name, import.version, cleanup_err
            ));
        }
        return Err(err);
    }

    Ok(())
}

fn copy_keg_dir(src: &Path, dest: &Path) -> Result<()> {
    #[cfg(not(unix))]
    {
        let _ = src;
        let _ = dest;
        return Err(ColdbrewError::Other(
            "Keg import is only supported on Unix platforms".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        if !src.exists() {
            return Err(ColdbrewError::PathNotFound(src.to_path_buf()));
        }
        if dest.exists() {
            return Err(ColdbrewError::Other(format!(
                "Destination already exists: {}",
                dest.display()
            )));
        }

        fs::create_dir_all(dest)?;
        copy_permissions(src, dest)?;

        for entry in WalkDir::new(src).follow_links(false) {
            let entry = entry?;
            let path = entry.path();
            let relative = match path.strip_prefix(src) {
                Ok(relative) if !relative.as_os_str().is_empty() => relative,
                _ => continue,
            };
            let target = dest.join(relative);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&target)?;
                copy_permissions(path, &target)?;
                continue;
            }

            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }

            if entry.file_type().is_symlink() {
                let link_target = fs::read_link(path)?;
                symlink(&link_target, &target)?;
                continue;
            }

            if entry.file_type().is_file() {
                fs::copy(path, &target)?;
                copy_permissions(path, &target)?;
            }
        }

        Ok(())
    }
}

fn copy_permissions(src: &Path, dest: &Path) -> Result<()> {
    let metadata = fs::metadata(src)?;
    fs::set_permissions(dest, metadata.permissions())?;
    Ok(())
}

pub async fn cleanup_brew_installs(
    brew_override: Option<&str>,
    migrated: &[MigratedFormula],
    output: &Output,
) -> Result<MigrationCleanupSummary> {
    if migrated.is_empty() {
        return Ok(MigrationCleanupSummary::new());
    }

    let brew = detect_brew(brew_override)?;
    let mut summary = MigrationCleanupSummary::new();

    for formula in migrated {
        output.info(&format!(
            "Removing Homebrew {}",
            Output::package_name(&formula.name)
        ));

        match run_brew(&brew, &["uninstall", "--formula", &formula.name]).await {
            Ok(_) => summary.removed.push(formula.name.clone()),
            Err(e) => summary.failed.push(MigrationFailure {
                name: formula.name.clone(),
                error: e.to_string(),
            }),
        }
    }

    Ok(summary)
}

fn detect_brew(brew_override: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = brew_override {
        let brew_path = PathBuf::from(path);
        if brew_path.exists() {
            return Ok(brew_path);
        }
        return Err(ColdbrewError::PathNotFound(brew_path));
    }

    if let Some(path) = find_in_path("brew") {
        return Ok(path);
    }

    let candidates = [
        "/opt/homebrew/bin/brew",
        "/usr/local/bin/brew",
        "/home/linuxbrew/.linuxbrew/bin/brew",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(ColdbrewError::HomebrewNotFound)
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for path in env::split_paths(&path_var) {
        let candidate = path.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

async fn run_brew(brew: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new(brew).args(args).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let command = args.join(" ");
        return Err(ColdbrewError::Other(format!(
            "Homebrew command failed: brew {} ({})",
            command,
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_brew_leaves(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

fn parse_brew_formula_versions(output: &str) -> HashMap<String, String> {
    let mut versions = HashMap::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();
        let name = match parts.next() {
            Some(name) => name,
            None => continue,
        };

        let version = match parts.last() {
            Some(version) => version,
            None => continue,
        };

        versions.insert(name.to_string(), version.to_string());
    }

    versions
}

fn parse_brew_casks(output: &str) -> Vec<String> {
    parse_brew_leaves(output)
}
