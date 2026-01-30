//! Space command - show disk usage and cleanup candidates

use crate::cli::output::{format_bytes, Output};
use crate::error::Result;
use crate::ops::cleanup::{self, CleanupCategory};
use crate::storage::Paths;

/// Execute the space command
pub async fn execute(details: bool, output: &Output) -> Result<()> {
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
