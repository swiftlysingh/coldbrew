//! Error types for Coldbrew

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for Coldbrew operations
#[derive(Error, Debug)]
pub enum ColdbrewError {
    #[error("Package '{0}' not found")]
    PackageNotFound(String),

    #[error("No bottle available for '{package}' on {platform}")]
    NoBottleAvailable { package: String, platform: String },

    #[error("Checksum mismatch for '{package}': expected {expected}, got {actual}")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    #[error("Package '{name}' version '{version}' is not installed")]
    PackageNotInstalled { name: String, version: String },

    #[error("Package '{name}' is already installed at version '{version}'")]
    PackageAlreadyInstalled { name: String, version: String },

    #[error(
        "Requested version '{requested}' for '{name}' is not available (current: {available})"
    )]
    VersionNotAvailable {
        name: String,
        requested: String,
        available: String,
    },

    #[error("Dependency '{dep}' required by '{package}' could not be resolved")]
    DependencyResolutionFailed { package: String, dep: String },

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Invalid version specification: '{0}'")]
    InvalidVersion(String),

    #[error("Unsupported platform: {os} {arch}")]
    UnsupportedPlatform { os: String, arch: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Tap '{0}' not found")]
    TapNotFound(String),

    #[error("Tap '{0}' already exists")]
    TapAlreadyExists(String),

    #[error("Invalid tap format: '{0}'. Expected 'user/repo'")]
    InvalidTapFormat(String),

    #[error("Lockfile not found. Run 'crew lock' first")]
    LockfileNotFound,

    #[error("Lockfile is out of sync with coldbrew.toml. Run 'crew lock' to update")]
    LockfileOutOfSync,

    #[error("Project file not found. Run 'crew init' first")]
    ProjectNotFound,

    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Failed to create directory: {0}")]
    DirectoryCreationFailed(PathBuf),

    #[error("Failed to extract archive: {0}")]
    ExtractionFailed(String),

    #[error("Cache is corrupted: {0}")]
    CacheCorrupted(String),

    #[error("Index is not initialized. Run 'crew update' first")]
    IndexNotInitialized,

    #[error("Index is stale. Run 'crew update' to refresh")]
    IndexStale,

    #[error("Package '{0}' is pinned and cannot be upgraded")]
    PackagePinned(String),

    #[error("No default version set for '{0}'")]
    NoDefaultVersion(String),

    #[error("GHCR authentication failed: {0}")]
    GhcrAuthFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Dialoguer error: {0}")]
    Dialoguer(#[from] dialoguer::Error),

    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("{0}")]
    Other(String),
}

/// Result type alias for Coldbrew operations
pub type Result<T> = std::result::Result<T, ColdbrewError>;

impl ColdbrewError {
    /// Returns a suggestion for how to fix the error, if applicable
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            ColdbrewError::PackageNotFound(_) => {
                Some("Try 'crew search <term>' to find available packages")
            }
            ColdbrewError::IndexNotInitialized | ColdbrewError::IndexStale => {
                Some("Run 'crew update' to fetch the latest package index")
            }
            ColdbrewError::LockfileNotFound => {
                Some("Run 'crew lock' to create a lockfile from coldbrew.toml")
            }
            ColdbrewError::LockfileOutOfSync => {
                Some("Run 'crew lock' to regenerate the lockfile from coldbrew.toml")
            }
            ColdbrewError::ProjectNotFound => {
                Some("Run 'crew init' to create a coldbrew.toml in this directory")
            }
            ColdbrewError::NoBottleAvailable { .. } => {
                Some("This package may require building from source, which is not yet supported")
            }
            ColdbrewError::PackagePinned(_) => Some("Use 'crew unpin <package>' to allow upgrades"),
            ColdbrewError::ChecksumMismatch { .. } => {
                Some("Try running 'crew clean' and retry the installation")
            }
            ColdbrewError::VersionNotAvailable { .. } => {
                Some("Run 'crew info <package>' to see the current available version")
            }
            _ => None,
        }
    }

    /// Returns true if this error is retryable (e.g., network errors)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ColdbrewError::Network(_)
                | ColdbrewError::DownloadFailed(_)
                | ColdbrewError::GhcrAuthFailed(_)
        )
    }
}
