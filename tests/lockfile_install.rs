//! Integration tests for lockfile-based installation

use coldbrew::config::{Lockfile, LockedPackage, ProjectConfig};
use coldbrew::error::ColdbrewError;
use std::collections::HashMap;
use tempfile::TempDir;

/// Test that --lock flag fails gracefully when no lockfile exists
#[test]
fn test_lockfile_not_found_error() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("coldbrew.lock");

    // Lockfile doesn't exist
    assert!(!lock_path.exists());

    // Load should fail
    let result = Lockfile::load(&lock_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        ColdbrewError::LockfileNotFound => {}
        other => panic!("Expected LockfileNotFound, got: {:?}", other),
    }
}

/// Test lockfile sync detection
#[test]
fn test_lockfile_out_of_sync() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("coldbrew.toml");
    let lock_path = temp.path().join("coldbrew.lock");

    // Create initial config
    let mut config = ProjectConfig::default();
    config.add_package("jq", "1.7", false);
    config.save(&config_path).unwrap();

    // Create lockfile with different hash
    let lockfile = Lockfile {
        version: 1,
        generated_at: chrono::Utc::now(),
        packages: HashMap::new(),
        config_hash: "old_hash_that_doesnt_match".to_string(),
    };
    lockfile.save(&lock_path).unwrap();

    // Load lockfile and check sync
    let loaded = Lockfile::load(&lock_path).unwrap();
    let config = ProjectConfig::load(&config_path).unwrap();

    assert!(!loaded.is_in_sync(&config), "Lockfile should be out of sync");
}

/// Test lockfile sync when in sync
#[test]
fn test_lockfile_in_sync() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("coldbrew.toml");
    let lock_path = temp.path().join("coldbrew.lock");

    // Create config
    let mut config = ProjectConfig::default();
    config.add_package("jq", "1.7", false);
    config.save(&config_path).unwrap();

    // Compute correct hash
    let config_content = toml::to_string(&config).unwrap();
    let config_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(config_content.as_bytes());
        hex::encode(hasher.finalize())
    };

    // Create lockfile with matching hash
    let lockfile = Lockfile {
        version: 1,
        generated_at: chrono::Utc::now(),
        packages: HashMap::new(),
        config_hash,
    };
    lockfile.save(&lock_path).unwrap();

    // Load and verify sync
    let loaded = Lockfile::load(&lock_path).unwrap();
    let config = ProjectConfig::load(&config_path).unwrap();

    assert!(loaded.is_in_sync(&config), "Lockfile should be in sync");
}

/// Test lockfile load/save roundtrip
#[test]
fn test_lockfile_roundtrip() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("coldbrew.lock");

    let mut packages = HashMap::new();
    packages.insert(
        "jq".to_string(),
        LockedPackage {
            version: "1.7.1".to_string(),
            sha256: Some("abc123".to_string()),
            bottle_tag: Some("arm64_sonoma".to_string()),
            tap: "homebrew/core".to_string(),
            dependencies: vec![],
            dev: false,
        },
    );

    let lockfile = Lockfile {
        version: 1,
        generated_at: chrono::Utc::now(),
        packages,
        config_hash: "test_hash".to_string(),
    };

    lockfile.save(&lock_path).unwrap();
    let loaded = Lockfile::load(&lock_path).unwrap();

    assert_eq!(loaded.version, 1);
    assert_eq!(loaded.packages.len(), 1);
    assert!(loaded.packages.contains_key("jq"));

    let jq = loaded.packages.get("jq").unwrap();
    assert_eq!(jq.version, "1.7.1");
    assert_eq!(jq.sha256, Some("abc123".to_string()));
}

/// Test error suggestions
#[test]
fn test_error_suggestions() {
    let err = ColdbrewError::LockfileNotFound;
    assert!(err.suggestion().is_some());
    assert!(err.suggestion().unwrap().contains("crew lock"));

    let err = ColdbrewError::LockfileOutOfSync;
    assert!(err.suggestion().is_some());
    assert!(err.suggestion().unwrap().contains("crew lock"));
}
