//! Command-line interface for Coldbrew

pub mod commands;
pub mod output;

use clap::{Parser, Subcommand};
use clap_complete::Shell;

/// Coldbrew - A Homebrew-compatible package manager
#[derive(Parser)]
#[command(name = "coldbrew")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Update the package index from Homebrew
    Update,

    /// Search for packages
    Search {
        /// Search query
        query: String,

        /// Show extended information
        #[arg(short, long)]
        extended: bool,
    },

    /// Show information about a package
    Info {
        /// Package name
        package: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Install packages
    Install {
        /// Packages to install (e.g., jq, node@22)
        #[arg(required = true)]
        packages: Vec<String>,

        /// Skip dependency installation
        #[arg(long)]
        skip_deps: bool,

        /// Force reinstall even if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall packages
    Uninstall {
        /// Packages to uninstall
        #[arg(required = true)]
        packages: Vec<String>,

        /// Remove all versions
        #[arg(short, long)]
        all: bool,

        /// Also remove unused dependencies
        #[arg(long)]
        with_deps: bool,
    },

    /// Upgrade installed packages
    Upgrade {
        /// Packages to upgrade (all if not specified)
        packages: Vec<String>,

        /// Skip interactive selection
        #[arg(short, long)]
        yes: bool,
    },

    /// List installed packages
    List {
        /// Show only package names
        #[arg(short, long)]
        names_only: bool,

        /// Show versions for a specific package
        #[arg(short, long)]
        versions: Option<String>,
    },

    /// Show which package provides a binary
    Which {
        /// Binary name
        binary: String,
    },

    /// Pin a package to prevent upgrades
    Pin {
        /// Package to pin
        package: String,
    },

    /// Unpin a package to allow upgrades
    Unpin {
        /// Package to unpin
        package: String,
    },

    /// Set or show the default version for a package
    Default {
        /// Package name (e.g., node@22 or just node to show current)
        package: String,
    },

    /// Show dependencies for a package
    Deps {
        /// Package name
        package: String,

        /// Show as tree
        #[arg(short, long)]
        tree: bool,
    },

    /// Show packages that depend on a package
    Dependents {
        /// Package name
        package: String,
    },

    /// Initialize a new coldbrew.toml in the current directory
    Init {
        /// Force overwrite if file exists
        #[arg(short, long)]
        force: bool,
    },

    /// Generate a lockfile from coldbrew.toml
    Lock,

    /// Add or remove taps (third-party repositories)
    Tap {
        /// Tap to add (user/repo format)
        tap: Option<String>,

        /// Remove a tap instead of adding
        #[arg(short, long)]
        remove: bool,
    },

    /// Manage the download cache
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },

    /// Garbage collection - remove old versions and orphan dependencies
    Gc {
        /// Dry run - show what would be removed
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Force-link a keg-only package
    Link {
        /// Package to link
        package: String,

        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
    },

    /// Remove links for a package
    Unlink {
        /// Package to unlink
        package: String,
    },

    /// Set up shell integration
    Shell {
        /// Shell to configure (bash, zsh, fish)
        shell: Option<String>,
    },

    /// Check system for potential problems
    Doctor,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },

    /// Execute a binary from a package (internal use)
    #[command(hide = true)]
    Exec {
        /// Package name
        package: String,

        /// Binary name
        binary: String,

        /// Arguments to pass to the binary
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// List cached downloads
    List,

    /// Remove cached downloads
    Clean {
        /// Remove all cached files
        #[arg(short, long)]
        all: bool,
    },

    /// Show cache location and size
    Info,
}

impl Cli {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
