//! Tap (third-party repository) management

use crate::error::{ColdbrewError, Result};
use crate::storage::Paths;
use git2::Repository;
use std::fs;
use std::path::PathBuf;

/// Information about an installed tap
#[derive(Debug, Clone)]
pub struct Tap {
    pub user: String,
    pub repo: String,
    pub path: PathBuf,
}

impl Tap {
    /// Get the full name (user/repo)
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.user, self.repo)
    }

    /// Get the GitHub URL
    pub fn github_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.user, self.repo)
    }
}

/// Manager for taps
pub struct TapManager {
    paths: Paths,
}

impl TapManager {
    /// Create a new TapManager
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    /// Parse a tap name (user/repo format)
    fn parse_tap_name(name: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() != 2 {
            return Err(ColdbrewError::InvalidTapFormat(name.to_string()));
        }

        let user = parts[0].to_string();
        let repo = parts[1].to_string();

        // Homebrew convention: if repo doesn't start with "homebrew-", add it
        let repo = if repo.starts_with("homebrew-") {
            repo
        } else {
            format!("homebrew-{}", repo)
        };

        Ok((user, repo))
    }

    /// Add a new tap
    pub async fn add(&mut self, name: &str) -> Result<Tap> {
        let (user, repo) = Self::parse_tap_name(name)?;
        let tap_dir = self.paths.tap_dir(&user, &repo);

        if tap_dir.exists() {
            return Err(ColdbrewError::TapAlreadyExists(format!(
                "{}/{}",
                user, repo
            )));
        }

        // Create parent directory
        if let Some(parent) = tap_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        // Clone the repository
        let url = format!("https://github.com/{}/{}.git", user, repo);

        // Use git2 for cloning
        Repository::clone(&url, &tap_dir)?;

        Ok(Tap {
            user,
            repo,
            path: tap_dir,
        })
    }

    /// Remove a tap
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let (user, repo) = Self::parse_tap_name(name)?;
        let tap_dir = self.paths.tap_dir(&user, &repo);

        if !tap_dir.exists() {
            return Err(ColdbrewError::TapNotFound(format!("{}/{}", user, repo)));
        }

        fs::remove_dir_all(&tap_dir)?;

        // Clean up empty parent directory
        let user_dir = self.paths.taps_dir().join(&user);
        if user_dir.exists() && user_dir.read_dir()?.next().is_none() {
            fs::remove_dir(&user_dir)?;
        }

        Ok(())
    }

    /// List all installed taps
    pub fn list(&self) -> Result<Vec<Tap>> {
        let taps_dir = self.paths.taps_dir();
        if !taps_dir.exists() {
            return Ok(Vec::new());
        }

        let mut taps = Vec::new();

        for user_entry in fs::read_dir(&taps_dir)? {
            let user_entry = user_entry?;
            if !user_entry.file_type()?.is_dir() {
                continue;
            }

            let user = user_entry.file_name().to_string_lossy().to_string();

            for repo_entry in fs::read_dir(user_entry.path())? {
                let repo_entry = repo_entry?;
                if !repo_entry.file_type()?.is_dir() {
                    continue;
                }

                let repo = repo_entry.file_name().to_string_lossy().to_string();

                taps.push(Tap {
                    user: user.clone(),
                    repo,
                    path: repo_entry.path(),
                });
            }
        }

        taps.sort_by_key(|tap| tap.full_name());
        Ok(taps)
    }

    /// Get a specific tap
    pub fn get(&self, name: &str) -> Result<Option<Tap>> {
        let (user, repo) = Self::parse_tap_name(name)?;
        let tap_dir = self.paths.tap_dir(&user, &repo);

        if !tap_dir.exists() {
            return Ok(None);
        }

        Ok(Some(Tap {
            user,
            repo,
            path: tap_dir,
        }))
    }

    /// Update a tap (git pull)
    pub fn update(&self, name: &str) -> Result<()> {
        let (user, repo) = Self::parse_tap_name(name)?;
        let tap_dir = self.paths.tap_dir(&user, &repo);

        if !tap_dir.exists() {
            return Err(ColdbrewError::TapNotFound(format!("{}/{}", user, repo)));
        }

        let repo = Repository::open(&tap_dir)?;

        // Fetch from origin
        let mut remote = repo.find_remote("origin")?;
        remote.fetch(&["main", "master"], None, None)?;

        // Get the default branch
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

        // Fast-forward merge
        let mut reference = repo.head()?;
        reference.set_target(fetch_commit.id(), "crew tap update")?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

        Ok(())
    }

    /// Update all taps
    pub fn update_all(&self) -> Result<Vec<String>> {
        let taps = self.list()?;
        let mut updated = Vec::new();

        for tap in taps {
            match self.update(&tap.full_name()) {
                Ok(_) => updated.push(tap.full_name()),
                Err(e) => {
                    tracing::warn!("Failed to update tap {}: {}", tap.full_name(), e);
                }
            }
        }

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_tap_name() {
        let (user, repo) = TapManager::parse_tap_name("user/repo").unwrap();
        assert_eq!(user, "user");
        assert_eq!(repo, "homebrew-repo");

        let (user, repo) = TapManager::parse_tap_name("user/homebrew-repo").unwrap();
        assert_eq!(user, "user");
        assert_eq!(repo, "homebrew-repo");
    }

    #[test]
    fn test_invalid_tap_name() {
        let result = TapManager::parse_tap_name("invalid");
        assert!(matches!(result, Err(ColdbrewError::InvalidTapFormat(_))));
    }

    #[test]
    fn test_list_empty() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        let manager = TapManager::new(paths);

        let taps = manager.list().unwrap();
        assert!(taps.is_empty());
    }
}
