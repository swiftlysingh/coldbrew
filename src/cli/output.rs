//! Terminal output formatting

use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};

/// Output formatter for CLI
pub struct Output {
    term: Term,
    quiet: bool,
    verbose: bool,
}

impl Output {
    /// Create a new output formatter
    pub fn new(quiet: bool, verbose: bool) -> Self {
        Self {
            term: Term::stderr(),
            quiet,
            verbose,
        }
    }

    /// Print an info message
    pub fn info(&self, message: &str) {
        if !self.quiet {
            let _ = self
                .term
                .write_line(&format!("{} {}", style("==>").cyan().bold(), message));
        }
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        if !self.quiet {
            let _ = self
                .term
                .write_line(&format!("{} {}", style("✓").green().bold(), message));
        }
    }

    /// Print a warning message
    pub fn warning(&self, message: &str) {
        let _ = self.term.write_line(&format!(
            "{} {}",
            style("Warning:").yellow().bold(),
            message
        ));
    }

    /// Print an error message
    pub fn error(&self, message: &str) {
        let _ = self
            .term
            .write_line(&format!("{} {}", style("Error:").red().bold(), message));
    }

    /// Print a verbose/debug message
    pub fn debug(&self, message: &str) {
        if self.verbose && !self.quiet {
            let _ = self
                .term
                .write_line(&format!("{} {}", style("DEBUG:").dim(), message));
        }
    }

    /// Print a plain message
    pub fn print(&self, message: &str) {
        if !self.quiet {
            println!("{}", message);
        }
    }

    /// Print a styled package name
    pub fn package_name(name: &str) -> String {
        style(name).green().bold().to_string()
    }

    /// Print a styled version
    pub fn version(version: &str) -> String {
        style(version).cyan().to_string()
    }

    /// Create a progress bar for downloads
    pub fn download_progress(&self, total: u64, message: &str) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(message.to_string());
        pb
    }

    /// Create a spinner for indeterminate progress
    pub fn spinner(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    }

    /// Print a table header
    pub fn table_header(&self, columns: &[(&str, usize)]) {
        if self.quiet {
            return;
        }

        let header: String = columns
            .iter()
            .map(|(name, width)| format!("{:width$}", name, width = *width))
            .collect::<Vec<_>>()
            .join("  ");

        println!("{}", style(&header).bold());
        println!("{}", style("-".repeat(header.len())).dim());
    }

    /// Print a table row
    pub fn table_row(&self, values: &[(&str, usize)]) {
        if self.quiet {
            return;
        }

        let row: String = values
            .iter()
            .map(|(value, width)| format!("{:width$}", value, width = *width))
            .collect::<Vec<_>>()
            .join("  ");

        println!("{}", row);
    }

    /// Print package info in a formatted way
    pub fn package_info(&self, name: &str, version: &str, desc: Option<&str>) {
        if self.quiet {
            return;
        }

        println!("{} {}", style(name).green().bold(), style(version).cyan());

        if let Some(desc) = desc {
            println!("  {}", desc);
        }
    }

    /// Print a list item
    pub fn list_item(&self, item: &str, details: Option<&str>) {
        if self.quiet {
            return;
        }

        print!("  {} {}", style("•").dim(), item);
        if let Some(details) = details {
            print!(" {}", style(details).dim());
        }
        println!();
    }

    /// Print a section header
    pub fn section(&self, title: &str) {
        if !self.quiet {
            println!();
            println!("{}", style(title).bold().underlined());
        }
    }

    /// Print caveats
    pub fn caveats(&self, caveats: &str) {
        if !self.quiet {
            println!();
            println!("{}", style("==> Caveats").yellow().bold());
            for line in caveats.lines() {
                println!("  {}", line);
            }
        }
    }

    /// Print a hint/suggestion
    pub fn hint(&self, message: &str) {
        if !self.quiet {
            println!("{} {}", style("Hint:").blue(), message);
        }
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(false, false)
    }
}

/// Format a duration as human-readable
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Format bytes as human-readable
pub fn format_bytes(bytes: u64) -> String {
    crate::storage::cache::format_bytes(bytes)
}
