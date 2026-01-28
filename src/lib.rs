//! Coldbrew - A Homebrew-compatible package manager
//!
//! Coldbrew provides a fast, reproducible, and user-controlled way to manage
//! Homebrew packages. It supports multiple versions, project-specific configurations,
//! and shim-based version management.

pub mod cli;
pub mod config;
pub mod core;
pub mod error;
pub mod ops;
pub mod registry;
pub mod storage;

pub use error::{ColdbrewError, Result};
