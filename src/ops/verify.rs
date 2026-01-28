//! Checksum verification

use crate::error::{ColdbrewError, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Verify the SHA256 checksum of a file
pub fn verify_sha256(path: &Path, expected: &str) -> Result<bool> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let actual = hex::encode(hasher.finalize());
    Ok(actual == expected.to_lowercase())
}

/// Calculate the SHA256 checksum of a file
pub fn calculate_sha256(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Verify a bottle file and return an error if verification fails
pub fn verify_bottle(path: &Path, expected: &str, package_name: &str) -> Result<()> {
    if !verify_sha256(path, expected)? {
        let actual = calculate_sha256(path)?;
        return Err(ColdbrewError::ChecksumMismatch {
            package: package_name.to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sha256_verification() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content").unwrap();

        // SHA256 of "test content"
        let expected = "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72";

        assert!(verify_sha256(file.path(), expected).unwrap());
        assert!(!verify_sha256(file.path(), "wrong_hash").unwrap());
    }

    #[test]
    fn test_calculate_sha256() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content").unwrap();

        let hash = calculate_sha256(file.path()).unwrap();
        assert_eq!(
            hash,
            "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72"
        );
    }
}
