//! Homebrew JSON API client

use crate::core::Formula;
use crate::error::{ColdbrewError, Result};
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use reqwest::Client;
use reqwest::StatusCode;
use std::time::Duration;

pub const FORMULA_INDEX_URL: &str = "https://formulae.brew.sh/api/formula.json";
const FORMULA_URL: &str = "https://formulae.brew.sh/api/formula";

#[derive(Debug, Clone)]
pub struct CacheHeaders {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug)]
pub enum IndexFetchResult {
    NotModified {
        cache: CacheHeaders,
    },
    Updated {
        formulas: Vec<Formula>,
        cache: CacheHeaders,
    },
}

/// Client for the Homebrew Formulae API
pub struct HomebrewApi {
    client: Client,
}

impl HomebrewApi {
    /// Create a new API client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .gzip(true)
            .pool_max_idle_per_host(6)
            .pool_idle_timeout(Duration::from_secs(60))
            .http2_adaptive_window(true)
            .build()?;

        Ok(Self { client })
    }

    /// Fetch the complete formula index
    pub async fn fetch_all_formulas(
        &self,
        cache: Option<&CacheHeaders>,
    ) -> Result<IndexFetchResult> {
        let mut request = self.client.get(FORMULA_INDEX_URL);
        if let Some(cache) = cache {
            if let Some(ref etag) = cache.etag {
                request = request.header(IF_NONE_MATCH, etag);
            }
            if let Some(ref last_modified) = cache.last_modified {
                request = request.header(IF_MODIFIED_SINCE, last_modified);
            }
        }

        let response = request.send().await?;
        let cache_headers = cache_headers_from_response(&response, cache);

        if response.status() == StatusCode::NOT_MODIFIED {
            return Ok(IndexFetchResult::NotModified {
                cache: cache_headers,
            });
        }

        if !response.status().is_success() {
            return Err(ColdbrewError::DownloadFailed(format!(
                "Failed to fetch formula index: {}",
                response.status()
            )));
        }

        let formulas: Vec<Formula> = response.json().await?;
        Ok(IndexFetchResult::Updated {
            formulas,
            cache: cache_headers,
        })
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

fn cache_headers_from_response(
    response: &reqwest::Response,
    fallback: Option<&CacheHeaders>,
) -> CacheHeaders {
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let last_modified = response
        .headers()
        .get(LAST_MODIFIED)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let mut headers = CacheHeaders {
        etag,
        last_modified,
    };
    if let Some(fallback) = fallback {
        if headers.etag.is_none() {
            headers.etag = fallback.etag.clone();
        }
        if headers.last_modified.is_none() {
            headers.last_modified = fallback.last_modified.clone();
        }
    }

    headers
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
