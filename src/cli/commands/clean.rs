//! Clean command - cleanup packages and cache

use crate::cli::output::{format_bytes, Output};
use crate::error::Result;
use crate::storage::{Cache, Cellar, Paths};
use std::time::Duration;

/// Execute the clean command
pub async fn execute(dry_run: bool, cache_only: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cellar = Cellar::new(paths.clone());
    let cache = Cache::new(paths);

    let mut total_freed = 0u64;
    let mut items_removed = 0usize;

    let mut to_remove = Vec::new();

    if !cache_only {
        // Find old versions (keep only latest 2 per package)
        output.info("Checking for old package versions...");

        let packages = cellar.list_packages()?;
        let mut by_name: std::collections::HashMap<String, Vec<_>> =
            std::collections::HashMap::new();

        for pkg in packages {
            by_name
                .entry(pkg.name.clone())
                .or_default()
                .push((pkg.version.clone(), pkg.cellar_path.clone()));
        }

        for (name, mut versions) in by_name {
            // Sort by version (newest last)
            versions.sort_by(|a, b| a.0.cmp(&b.0));

            // Keep last 2 versions
            if versions.len() > 2 {
                let old_versions = &versions[..versions.len() - 2];
                for (version, path) in old_versions {
                    // Calculate size
                    let size = walkdir::WalkDir::new(path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                        .filter_map(|e| e.metadata().ok())
                        .map(|m| m.len())
                        .sum::<u64>();

                    to_remove.push((name.clone(), version.clone(), path.clone(), size));
                }
            }
        }

        if !to_remove.is_empty() {
            output.section("Old versions to remove");
            for (name, version, path, size) in &to_remove {
                println!(
                    "  {} {} - {}",
                    Output::package_name(name),
                    Output::version(version),
                    format_bytes(*size)
                );

                if !dry_run {
                    std::fs::remove_dir_all(path)?;
                    total_freed += size;
                    items_removed += 1;
                }
            }
        }
    }

    // Clean old cache files (older than 30 days)
    output.info("Checking cache...");

    if !dry_run {
        let cache_result = cache.clean(Some(Duration::from_secs(30 * 24 * 60 * 60)))?;
        total_freed += cache_result.freed;
        items_removed += cache_result.removed;

        if cache_result.removed > 0 {
            output.info(&format!(
                "Cleaned {} cached files ({})",
                cache_result.removed,
                cache_result.freed_human()
            ));
        }
    }

    // Summary
    println!();
    if dry_run {
        output.info("Dry run - no changes made");
        if !to_remove.is_empty() {
            let total_size: u64 = to_remove.iter().map(|(_, _, _, s)| s).sum();
            output.info(&format!(
                "Would remove {} items, freeing {}",
                to_remove.len(),
                format_bytes(total_size)
            ));
        }
    } else if items_removed == 0 {
        output.success("Nothing to clean up");
    } else {
        output.success(&format!(
            "Removed {} items, freed {}",
            items_removed,
            format_bytes(total_freed)
        ));
    }

    Ok(())
}
