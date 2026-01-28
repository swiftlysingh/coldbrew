//! Homebrew JSON API client

use crate::core::Formula;
use crate::error::{ColdbrewError, Result};
use reqwest::Client;
use std::time::Duration;

const FORMULA_INDEX_URL: &str = "https://formulae.brew.sh/api/formula.json";
const FORMULA_URL: &str = "https://formulae.brew.sh/api/formula";

/// Client for the Homebrew Formulae API
pub struct HomebrewApi {
    client: Client,
}

impl HomebrewApi {
    /// Create a new API client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .gzip(true)
            .build()?;

        Ok(Self { client })
    }

    /// Fetch the complete formula index
    pub async fn fetch_all_formulas(&self) -> Result<Vec<Formula>> {
        let response = self.client.get(FORMULA_INDEX_URL).send().await?;

        if !response.status().is_success() {
            return Err(ColdbrewError::DownloadFailed(format!(
                "Failed to fetch formula index: {}",
                response.status()
            )));
        }

        let formulas: Vec<Formula> = response.json().await?;
        Ok(formulas)
    }

    /// Fetch a single formula by name
    pub async fn fetch_formula(&self, name: &str) -> Result<Formula> {
        let url = format!("{}/{}.json", FORMULA_URL, name);
        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ColdbrewError::PackageNotFound(name.to_string()));
        }

        if !response.status().is_success() {
            return Err(ColdbrewError::DownloadFailed(format!(
                "Failed to fetch formula {}: {}",
                name,
                response.status()
            )));
        }

        let formula: Formula = response.json().await?;
        Ok(formula)
    }

    /// Get the size of the formula index (for progress reporting)
    pub async fn get_index_size(&self) -> Result<u64> {
        let response = self.client.head(FORMULA_INDEX_URL).send().await?;

        Ok(response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(15_000_000)) // Default to ~15MB
    }
}

impl Default for HomebrewApi {
    fn default() -> Self {
        Self::new().expect("Failed to create HTTP client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_fetch_formula() {
        let api = HomebrewApi::new().unwrap();
        let formula = api.fetch_formula("jq").await.unwrap();
        assert_eq!(formula.name, "jq");
    }
}
