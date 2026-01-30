//! Local formula index cache

use crate::core::Formula;
use crate::error::{ColdbrewError, Result};
use crate::registry::homebrew_api::{CacheHeaders, IndexFetchResult, FORMULA_INDEX_URL};
use crate::registry::HomebrewApi;
use crate::storage::{Database, Paths};
use std::collections::HashMap;
use std::fs;

/// Local cache of the Homebrew formula index
pub struct Index {
    paths: Paths,
    formulas: Option<HashMap<String, Formula>>,
}

impl Index {
    /// Create a new Index
    pub fn new(paths: Paths) -> Self {
        Self {
            paths,
            formulas: None,
        }
    }

    /// Update the index from the Homebrew API
    pub async fn update(&mut self) -> Result<usize> {
        let api = HomebrewApi::new()?;
        let db = Database::new(self.paths.clone());
        let conn = db.connect()?;

        let index_path = self.paths.formula_index();
        let cache_entry = if index_path.exists() {
            db.get_api_cache(&conn, FORMULA_INDEX_URL)?
        } else {
            None
        };
        let cache_headers = cache_entry.as_ref().map(|entry| CacheHeaders {
            etag: entry.etag.clone(),
            last_modified: entry.last_modified.clone(),
        });

        let fetch = api.fetch_all_formulas(cache_headers.as_ref()).await?;
        let (count, cache_update) = match fetch {
            IndexFetchResult::Updated { formulas, cache } => {
                if let Some(parent) = index_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let json = serde_json::to_string(&formulas)?;
                fs::write(&index_path, json)?;

                let mut map = HashMap::new();
                for formula in formulas {
                    map.insert(formula.name.clone(), formula);
                }
                let count = map.len();
                self.formulas = Some(map);
                (count, cache)
            }
            IndexFetchResult::NotModified { cache } => {
                self.load()?;
                let count = self.formulas.as_ref().map(|map| map.len()).unwrap_or(0);
                (count, cache)
            }
        };

        db.upsert_api_cache(
            &conn,
            FORMULA_INDEX_URL,
            cache_update.etag.as_deref(),
            cache_update.last_modified.as_deref(),
        )?;

        Ok(count)
    }

    /// Load the index from disk
    #[allow(dead_code)]
    fn load(&mut self) -> Result<()> {
        if self.formulas.is_some() {
            return Ok(());
        }

        let index_path = self.paths.formula_index();
        if !index_path.exists() {
            return Err(ColdbrewError::IndexNotInitialized);
        }

        let content = fs::read_to_string(&index_path)?;
        let formulas: Vec<Formula> = serde_json::from_str(&content)?;

        let mut map = HashMap::new();
        for formula in formulas {
            map.insert(formula.name.clone(), formula);
        }
        self.formulas = Some(map);

        Ok(())
    }

    /// Get a formula by name
    pub fn get_formula(&self, name: &str) -> Result<Option<Formula>> {
        let index_path = self.paths.formula_index();
        if !index_path.exists() {
            return Err(ColdbrewError::IndexNotInitialized);
        }

        // Load from disk if not cached
        let content = fs::read_to_string(&index_path)?;
        let formulas: Vec<Formula> = serde_json::from_str(&content)?;

        Ok(formulas.into_iter().find(|f| f.name == name))
    }

    /// Search for formulas matching a query
    pub fn search(&self, query: &str) -> Result<Vec<Formula>> {
        let index_path = self.paths.formula_index();
        if !index_path.exists() {
            return Err(ColdbrewError::IndexNotInitialized);
        }

        let content = fs::read_to_string(&index_path)?;
        let formulas: Vec<Formula> = serde_json::from_str(&content)?;

        let query_lower = query.to_lowercase();
        let mut results: Vec<Formula> = formulas
            .into_iter()
            .filter(|f| {
                f.name.to_lowercase().contains(&query_lower)
                    || f.desc
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect();

        // Sort by relevance (exact name match first, then name contains, then description)
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;
            let a_starts = a.name.to_lowercase().starts_with(&query_lower);
            let b_starts = b.name.to_lowercase().starts_with(&query_lower);

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a_starts, b_starts) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                },
            }
        });

        Ok(results)
    }

    /// List all formulas
    pub fn list_formulas(&self) -> Result<Vec<Formula>> {
        let index_path = self.paths.formula_index();
        if !index_path.exists() {
            return Err(ColdbrewError::IndexNotInitialized);
        }

        let content = fs::read_to_string(&index_path)?;
        let formulas: Vec<Formula> = serde_json::from_str(&content)?;
        Ok(formulas)
    }

    /// Check if the index exists
    pub fn exists(&self) -> bool {
        self.paths.formula_index().exists()
    }

    /// Get the age of the index in seconds
    pub fn age_seconds(&self) -> Result<u64> {
        let index_path = self.paths.formula_index();
        if !index_path.exists() {
            return Err(ColdbrewError::IndexNotInitialized);
        }

        let metadata = fs::metadata(&index_path)?;
        let modified = metadata.modified()?;
        let age = std::time::SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default();

        Ok(age.as_secs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_index_not_initialized() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        let index = Index::new(paths);

        let result = index.get_formula("jq");
        assert!(matches!(result, Err(ColdbrewError::IndexNotInitialized)));
    }
}
