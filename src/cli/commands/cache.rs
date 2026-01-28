//! Cache command - manage download cache

use crate::cli::output::{format_bytes, Output};
use crate::error::Result;
use crate::storage::{Cache, Paths};
use std::time::Duration;

/// Execute the cache list command
pub async fn execute_list(output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cache = Cache::new(paths);

    let bottles = cache.list()?;

    if bottles.is_empty() {
        output.info("Cache is empty");
        return Ok(());
    }

    output.info(&format!("{} items in cache:", bottles.len()));
    println!();

    for bottle in bottles {
        println!(
            "  {} {} ({}) - {}",
            Output::package_name(&bottle.name),
            Output::version(&bottle.version),
            bottle.tag,
            format_bytes(bottle.size)
        );
    }

    println!();
    let total = cache.total_size()?;
    output.info(&format!("Total: {}", format_bytes(total)));

    Ok(())
}

/// Execute the cache clean command
pub async fn execute_clean(all: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cache = Cache::new(paths);

    let max_age = if all {
        None
    } else {
        Some(Duration::from_secs(7 * 24 * 60 * 60)) // 7 days
    };

    let result = cache.clean(max_age)?;

    if result.removed == 0 {
        output.info("Nothing to clean");
    } else {
        output.success(&format!(
            "Removed {} items, freed {}",
            result.removed,
            result.freed_human()
        ));
    }

    Ok(())
}

/// Execute the cache info command
pub async fn execute_info(output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let cache = Cache::new(paths.clone());

    let total = cache.total_size()?;
    let count = cache.list()?.len();

    output.info("Cache information:");
    println!("  Location: {}", paths.downloads_dir().display());
    println!("  Items: {}", count);
    println!("  Size: {}", format_bytes(total));

    Ok(())
}
