//! Homebrew migration operations

use crate::cli::output::Output;
use crate::error::{ColdbrewError, Result};
use crate::ops;
use crate::registry::Index;
use crate::storage::{Cellar, Paths};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use tokio::process::Command;

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
pub struct MigrationSummary {
    pub requested: usize,
    pub migrated: Vec<String>,
    pub skipped: Vec<MigrationSkip>,
    pub failed: Vec<MigrationFailure>,
    pub casks: Vec<String>,
    pub warnings: Vec<String>,
    pub dry_run: bool,
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

        if formula.versions.stable != *installed_version {
            summary.skipped.push(MigrationSkip {
                name: name.clone(),
                reason: format!(
                    "Installed version {} does not match current stable {}",
                    installed_version, formula.versions.stable
                ),
            });
            continue;
        }

        if cellar.is_installed(&name, installed_version) {
            summary.skipped.push(MigrationSkip {
                name: name.clone(),
                reason: "Already installed in Coldbrew".to_string(),
            });
            continue;
        }

        if dry_run {
            output.info(&format!(
                "Would migrate {} {}",
                Output::package_name(&name),
                Output::version(installed_version)
            ));
            summary
                .migrated
                .push(format!("{} {}", name, installed_version));
            continue;
        }

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
                summary
                    .migrated
                    .push(format!("{} {}", name, installed_version));
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
