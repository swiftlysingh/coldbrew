//! Doctor command - system diagnostics

use crate::cli::output::{format_bytes, Output};
use crate::core::Platform;
use crate::error::Result;
use crate::storage::{Cache, Cellar, Paths, ShimManager};
use std::env;

/// Execute the doctor command
pub async fn execute(output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    output.info("Running diagnostics...\n");

    // Platform check
    check_platform()?;

    // PATH check
    check_path(&paths, output, &mut warnings);

    // Directory permissions
    check_permissions(&paths, output, &mut issues);

    // Shim integrity
    check_shims(&paths, output, &mut warnings);

    // Cache status
    check_cache(&paths, output, &mut warnings);

    // Disk space
    check_disk_space(&paths, output, &mut warnings);

    // Index freshness
    check_index(&paths, output, &mut warnings);

    // Summary
    println!();
    if issues.is_empty() && warnings.is_empty() {
        output.success("Your Coldbrew installation looks good!");
    } else {
        if !issues.is_empty() {
            output.error(&format!("{} issue(s) found:", issues.len()));
            for issue in &issues {
                println!("  {} {}", console::style("✗").red(), issue);
            }
        }

        if !warnings.is_empty() {
            output.warning(&format!("{} warning(s):", warnings.len()));
            for warning in &warnings {
                println!("  {} {}", console::style("!").yellow(), warning);
            }
        }
    }

    Ok(())
}

fn check_platform() -> Result<()> {
    let platform = Platform::detect()?;
    println!("  {} Platform: {}", console::style("✓").green(), platform);
    println!(
        "    Bottle tag: {}",
        console::style(platform.bottle_tag()).dim()
    );
    Ok(())
}

fn check_path(paths: &Paths, _output: &Output, warnings: &mut Vec<String>) {
    let bin_dir = paths.bin_dir();
    let path_var = env::var("PATH").unwrap_or_default();

    if path_var.contains(&bin_dir.to_string_lossy().to_string()) {
        println!(
            "  {} PATH: Coldbrew bin directory is in PATH",
            console::style("✓").green()
        );
    } else {
        warnings.push(
            "Coldbrew bin directory not in PATH. Run 'crew shell' for setup instructions"
                .to_string(),
        );
        println!(
            "  {} PATH: {} not in PATH",
            console::style("!").yellow(),
            bin_dir.display()
        );
    }
}

fn check_permissions(paths: &Paths, _output: &Output, issues: &mut Vec<String>) {
    let dirs = [
        ("Root", paths.root()),
        ("Cellar", &paths.cellar_dir()),
        ("Cache", &paths.cache_dir()),
    ];

    for (name, dir) in &dirs {
        if dir.exists() {
            match std::fs::metadata(dir) {
                Ok(meta) => {
                    use std::os::unix::fs::MetadataExt;
                    let mode = meta.mode();
                    let writable = (mode & 0o200) != 0;

                    if writable {
                        println!("  {} {}: OK", console::style("✓").green(), name);
                    } else {
                        issues.push(format!("{} directory is not writable", name));
                    }
                }
                Err(e) => {
                    issues.push(format!("Cannot access {} directory: {}", name, e));
                }
            }
        } else {
            println!("  {} {}: Not created yet", console::style("○").dim(), name);
        }
    }
}

fn check_shims(paths: &Paths, _output: &Output, warnings: &mut Vec<String>) {
    let shim_manager = ShimManager::new(paths.clone());

    match shim_manager.list_shims() {
        Ok(shims) => {
            let mut broken = 0;
            for shim in &shims {
                // Check if the target package still exists
                let cellar = Cellar::new(paths.clone());
                if cellar
                    .get_versions(&shim.package)
                    .unwrap_or_default()
                    .is_empty()
                {
                    broken += 1;
                }
            }

            if broken > 0 {
                warnings.push(format!(
                    "{} broken shim(s) found. Run 'crew gc' to clean up",
                    broken
                ));
                println!(
                    "  {} Shims: {} total, {} broken",
                    console::style("!").yellow(),
                    shims.len(),
                    broken
                );
            } else {
                println!(
                    "  {} Shims: {} installed",
                    console::style("✓").green(),
                    shims.len()
                );
            }
        }
        Err(e) => {
            warnings.push(format!("Could not check shims: {}", e));
        }
    }
}

fn check_cache(paths: &Paths, _output: &Output, warnings: &mut Vec<String>) {
    let cache = Cache::new(paths.clone());

    match cache.total_size() {
        Ok(size) => {
            // Warn if cache is over 1GB
            if size > 1024 * 1024 * 1024 {
                warnings.push(format!(
                    "Cache is large ({}). Consider running 'crew cache clean'",
                    format_bytes(size)
                ));
                println!(
                    "  {} Cache: {} (large)",
                    console::style("!").yellow(),
                    format_bytes(size)
                );
            } else {
                println!(
                    "  {} Cache: {}",
                    console::style("✓").green(),
                    format_bytes(size)
                );
            }
        }
        Err(e) => {
            warnings.push(format!("Could not check cache: {}", e));
        }
    }
}

#[cfg_attr(not(target_os = "macos"), allow(unused_variables, clippy::ptr_arg))]
fn check_disk_space(paths: &Paths, _output: &Output, warnings: &mut Vec<String>) {
    // Check available disk space on the coldbrew directory
    #[cfg(target_os = "macos")]
    {
        if let Ok(statvfs) = nix::sys::statvfs::statvfs(paths.root()) {
            let available = u64::from(statvfs.blocks_available()) * statvfs.block_size();

            if available < 1024 * 1024 * 1024 {
                // Less than 1GB
                warnings.push(format!(
                    "Low disk space: {} available",
                    format_bytes(available)
                ));
                println!(
                    "  {} Disk: {} available (low)",
                    console::style("!").yellow(),
                    format_bytes(available)
                );
            } else {
                println!(
                    "  {} Disk: {} available",
                    console::style("✓").green(),
                    format_bytes(available)
                );
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        println!(
            "  {} Disk: Check not available on this platform",
            console::style("○").dim()
        );
    }
}

fn check_index(paths: &Paths, _output: &Output, warnings: &mut Vec<String>) {
    let index_path = paths.formula_index();

    if !index_path.exists() {
        warnings.push("Package index not found. Run 'crew update'".to_string());
        println!("  {} Index: Not initialized", console::style("!").yellow());
        return;
    }

    match index_path.metadata() {
        Ok(meta) => {
            if let Ok(modified) = meta.modified() {
                let age = std::time::SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default();

                let days = age.as_secs() / 86400;

                if days > 7 {
                    warnings.push(format!(
                        "Package index is {} days old. Consider running 'crew update'",
                        days
                    ));
                    println!(
                        "  {} Index: {} days old (stale)",
                        console::style("!").yellow(),
                        days
                    );
                } else {
                    println!("  {} Index: {} days old", console::style("✓").green(), days);
                }
            }
        }
        Err(e) => {
            warnings.push(format!("Could not check index: {}", e));
        }
    }
}
