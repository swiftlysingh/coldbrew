//! GitHub Container Registry client for bottle downloads

use crate::core::{BottleFile, Formula};
use crate::error::{ColdbrewError, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const GHCR_TOKEN_URL: &str = "https://ghcr.io/token";

/// Client for downloading bottles from GitHub Container Registry
pub struct GhcrClient {
    client: Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
}

#[derive(Clone)]
struct TokenCache {
    token: String,
    repository: String,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
    expires_in: Option<u64>,
}

impl GhcrClient {
    /// Create a new GHCR client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;

        Ok(Self {
            client,
            token_cache: Arc::new(RwLock::new(None)),
        })
    }

    fn repository_from_url(url: &str) -> Result<String> {
        let parsed = reqwest::Url::parse(url).map_err(|err| {
            ColdbrewError::GhcrAuthFailed(format!("Invalid bottle URL: {}", err))
        })?;
        let path = parsed.path();
        let prefix = "/v2/";
        let blobs = "/blobs/";

        let start = path.find(prefix).ok_or_else(|| {
            ColdbrewError::GhcrAuthFailed("Bottle URL missing /v2/ segment".to_string())
        })?;
        let after = &path[start + prefix.len()..];
        let end = after.find(blobs).ok_or_else(|| {
            ColdbrewError::GhcrAuthFailed("Bottle URL missing /blobs/ segment".to_string())
        })?;

        let repository = &after[..end];
        if repository.is_empty() {
            return Err(ColdbrewError::GhcrAuthFailed(
                "Bottle URL has empty repository".to_string(),
            ));
        }

        Ok(repository.to_string())
    }

    /// Get a bearer token for a repository
    async fn get_token(&self, repository: &str) -> Result<String> {
        // Check cache
        {
            let cache = self.token_cache.read().await;
            if let Some(ref cached) = *cache {
                if cached.repository == repository && cached.expires_at > Instant::now() {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Fetch new token
        let scope = format!("repository:{}:pull", repository);

        let response = self
            .client
            .get(GHCR_TOKEN_URL)
            .query(&[("service", "ghcr.io"), ("scope", &scope)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let detail = body.trim();
            let message = if detail.is_empty() {
                format!("Token request failed: {}", status)
            } else {
                format!("Token request failed: {} {}", status, detail)
            };
            return Err(ColdbrewError::GhcrAuthFailed(message));
        }

        let token_response: TokenResponse = response.json().await?;
        let expires_in = token_response.expires_in.unwrap_or(300); // Default 5 minutes

        // Cache the token
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(TokenCache {
                token: token_response.token.clone(),
                repository: repository.to_string(),
                expires_at: Instant::now() + Duration::from_secs(expires_in - 30), // Buffer
            });
        }

        Ok(token_response.token)
    }

    /// Download a bottle to a file
    pub async fn download_bottle<F>(
        &self,
        _formula: &Formula,
        bottle_file: &BottleFile,
        dest: &Path,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(u64, u64),
    {
        let mut refreshed = false;
        let repository = Self::repository_from_url(&bottle_file.url)?;

        loop {
            let token = self.get_token(&repository).await?;

            let response = self
                .client
                .get(&bottle_file.url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;

            if response.status() == reqwest::StatusCode::UNAUTHORIZED && !refreshed {
                {
                    let mut cache = self.token_cache.write().await;
                    *cache = None;
                }
                refreshed = true;
                continue;
            }

            if !response.status().is_success() {
                return Err(ColdbrewError::DownloadFailed(format!(
                    "Bottle download failed: {}",
                    response.status()
                )));
            }

            let total_size = response.content_length().unwrap_or(0);
            let mut downloaded: u64 = 0;

            let mut file = std::fs::File::create(dest)?;
            let mut stream = response.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                file.write_all(&chunk)?;
                downloaded += chunk.len() as u64;
                progress_callback(downloaded, total_size);
            }

            file.flush()?;

            return Ok(());
        }
    }

    /// Download a bottle and return the bytes
    pub async fn download_bottle_bytes(
        &self,
        _formula: &Formula,
        bottle_file: &BottleFile,
    ) -> Result<Vec<u8>> {
        let repository = Self::repository_from_url(&bottle_file.url)?;
        let token = self.get_token(&repository).await?;

        let response = self
            .client
            .get(&bottle_file.url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ColdbrewError::DownloadFailed(format!(
                "Bottle download failed: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

impl Default for GhcrClient {
    fn default() -> Self {
        Self::new().expect("Failed to create GHCR client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_get_token() {
        let client = GhcrClient::new().unwrap();
        let token = client.get_token("homebrew/core/jq").await.unwrap();
        assert!(!token.is_empty());
    }
}
