//! Space command - disk usage and cleanup

use crate::cli::output::{format_bytes, Output};
use crate::error::Result;
use crate::ops::cleanup::{self, CleanupCategory, CleanupKind};
use crate::storage::Paths;
use dialoguer::Input;
use std::collections::HashSet;

/// Execute the space show command
pub async fn execute_show(details: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let categories = cleanup::collect_categories(&paths)?;

    output.section("Coldbrew disk usage");
    print_summary(&categories);

    if details {
        for category in &categories {
            if category.items.is_empty() {
                continue;
            }
            output.section(&format!("Details: {}", category.title));
            print_details(category);
        }
    }

    Ok(())
}

/// Execute the space clean command
pub async fn execute_clean(dry_run: bool, all: bool, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let categories = cleanup::collect_categories(&paths)?;

    let has_items = categories.iter().any(|category| !category.is_empty());
    if !has_items {
        output.success("Nothing to clean up");
        return Ok(());
    }

    output.section("Cleanup candidates");
    print_summary(&categories);

    let selected = if all {
        categories
            .iter()
            .filter(|category| !category.is_empty())
            .map(|category| category.kind)
            .collect::<HashSet<_>>()
    } else {
        match prompt_mode()? {
            CleanupMode::Quit => {
                output.info("Cleanup cancelled");
                return Ok(());
            }
            CleanupMode::All => categories
                .iter()
                .filter(|category| !category.is_empty())
                .map(|category| category.kind)
                .collect(),
            CleanupMode::Select => prompt_categories(&categories, output)?,
        }
    };

    if selected.is_empty() {
        output.info("No cleanup targets selected");
        return Ok(());
    }

    let result = cleanup::apply_cleanup(&paths, &categories, &selected, dry_run)?;

    println!();
    if dry_run {
        output.info("Dry run - no changes made");
        output.info(&format!(
            "Would remove {} items, freeing {}",
            result.removed,
            format_bytes(result.freed)
        ));
    } else if result.removed == 0 {
        output.success("Nothing to clean up");
    } else {
        output.success(&format!(
            "Removed {} items, freed {}",
            result.removed,
            format_bytes(result.freed)
        ));
    }

    Ok(())
}

fn print_summary(categories: &[CleanupCategory]) {
    for category in categories {
        let count = category.items.len();
        let size = format_bytes(category.total_size());
        println!("  {:<22} {:>3} items ({})", category.title, count, size);
    }
}

fn print_details(category: &CleanupCategory) {
    for item in &category.items {
        println!(
            "  {} - {} ({})",
            item.label,
            format_bytes(item.size),
            item.path.display()
        );
    }
}

fn prompt_mode() -> Result<CleanupMode> {
    loop {
        let input: String = Input::new()
            .with_prompt("Clean all? [a]ll / [s]elect / [q]uit")
            .default("s".to_string())
            .interact_text()?;

        match input.trim().to_lowercase().as_str() {
            "a" | "all" => return Ok(CleanupMode::All),
            "s" | "select" => return Ok(CleanupMode::Select),
            "q" | "quit" => return Ok(CleanupMode::Quit),
            _ => println!("  Please enter a, s, or q"),
        }
    }
}

fn prompt_categories(
    categories: &[CleanupCategory],
    output: &Output,
) -> Result<HashSet<CleanupKind>> {
    let mut selected = HashSet::new();

    for category in categories {
        if category.is_empty() {
            continue;
        }

        loop {
            let prompt = format!("Remove {}? [y/N/d]", category.title.to_lowercase());
            let input: String = Input::new()
                .with_prompt(prompt)
                .default("n".to_string())
                .interact_text()?;

            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => {
                    selected.insert(category.kind);
                    break;
                }
                "n" | "no" => break,
                "d" | "details" => {
                    output.section(&format!("Details: {}", category.title));
                    print_details(category);
                }
                _ => println!("  Please enter y, n, or d"),
            }
        }
    }

    Ok(selected)
}

#[derive(Debug, Clone, Copy)]
enum CleanupMode {
    All,
    Select,
    Quit,
}
