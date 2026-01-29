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
    package: String,
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

    /// Get a bearer token for a package
    async fn get_token(&self, package: &str) -> Result<String> {
        // Check cache
        {
            let cache = self.token_cache.read().await;
            if let Some(ref cached) = *cache {
                if cached.package == package && cached.expires_at > Instant::now() {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Fetch new token
        let scope = format!("repository:homebrew/core/{}:pull", package);
        let url = format!("{}?scope={}", GHCR_TOKEN_URL, scope);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ColdbrewError::GhcrAuthFailed(format!(
                "Token request failed: {}",
                response.status()
            )));
        }

        let token_response: TokenResponse = response.json().await?;
        let expires_in = token_response.expires_in.unwrap_or(300); // Default 5 minutes

        // Cache the token
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(TokenCache {
                token: token_response.token.clone(),
                package: package.to_string(),
                expires_at: Instant::now() + Duration::from_secs(expires_in - 30), // Buffer
            });
        }

        Ok(token_response.token)
    }

    /// Download a bottle to a file
    pub async fn download_bottle<F>(
        &self,
        formula: &Formula,
        bottle_file: &BottleFile,
        dest: &Path,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(u64, u64),
    {
        let mut refreshed = false;

        loop {
            let token = self.get_token(&formula.name).await?;

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
        formula: &Formula,
        bottle_file: &BottleFile,
    ) -> Result<Vec<u8>> {
        let token = self.get_token(&formula.name).await?;

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
        let token = client.get_token("jq").await.unwrap();
        assert!(!token.is_empty());
    }
}
