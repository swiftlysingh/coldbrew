use crate::error::Result;
use crate::storage::Paths;
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i32 = 3;

/// SQLite-backed metadata store
pub struct Database {
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ApiCacheEntry {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub cached_at: i64,
}

#[derive(Debug, Clone)]
pub struct BlobCacheEntry {
    pub sha256: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub tag: Option<String>,
    pub size_bytes: u64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct StoreEntryInfo {
    pub sha256: String,
    pub size_bytes: u64,
    pub created_at: i64,
}

impl Database {
    /// Create a new Database handle
    pub fn new(paths: Paths) -> Self {
        Self {
            path: paths.db_file(),
        }
    }

    /// Open a connection and ensure the schema exists
    pub fn connect(&self) -> Result<Connection> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&self.path)?;
        Self::configure(&conn)?;
        Self::migrate(&conn)?;
        Ok(conn)
    }

    /// Get a cached API entry by URL
    pub fn get_api_cache(&self, conn: &Connection, url: &str) -> Result<Option<ApiCacheEntry>> {
        let mut stmt =
            conn.prepare("SELECT etag, last_modified, cached_at FROM api_cache WHERE url = ?1")?;
        let entry = stmt
            .query_row(params![url], |row| {
                Ok(ApiCacheEntry {
                    etag: row.get(0)?,
                    last_modified: row.get(1)?,
                    cached_at: row.get(2)?,
                })
            })
            .optional()?;
        Ok(entry)
    }

    /// Insert or update API cache headers
    pub fn upsert_api_cache(
        &self,
        conn: &Connection,
        url: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<()> {
        let cached_at = now_timestamp();
        conn.execute(
            "INSERT INTO api_cache (url, etag, last_modified, cached_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(url) DO UPDATE
             SET etag = excluded.etag,
                 last_modified = excluded.last_modified,
                 cached_at = excluded.cached_at",
            params![url, etag, last_modified, cached_at],
        )?;
        Ok(())
    }

    pub fn upsert_blob_cache(
        &self,
        conn: &Connection,
        sha256: &str,
        name: Option<&str>,
        version: Option<&str>,
        tag: Option<&str>,
        size_bytes: u64,
    ) -> Result<()> {
        let created_at = now_timestamp();
        conn.execute(
            "INSERT INTO blob_cache (sha256, name, version, tag, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(sha256) DO UPDATE
             SET name = excluded.name,
                 version = excluded.version,
                 tag = excluded.tag,
                 size_bytes = excluded.size_bytes,
                 created_at = excluded.created_at",
            params![sha256, name, version, tag, size_bytes, created_at],
        )?;
        Ok(())
    }

    pub fn delete_blob_cache(&self, conn: &Connection, sha256: &str) -> Result<()> {
        conn.execute("DELETE FROM blob_cache WHERE sha256 = ?1", params![sha256])?;
        Ok(())
    }

    pub fn list_blob_cache(&self, conn: &Connection) -> Result<Vec<BlobCacheEntry>> {
        let mut stmt = conn.prepare(
            "SELECT sha256, name, version, tag, size_bytes, created_at
             FROM blob_cache
             ORDER BY name, version",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BlobCacheEntry {
                sha256: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                tag: row.get(3)?,
                size_bytes: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in rows {
            entries.push(entry?);
        }
        Ok(entries)
    }

    pub fn upsert_store_entry(
        &self,
        conn: &Connection,
        sha256: &str,
        size_bytes: u64,
    ) -> Result<()> {
        let created_at = now_timestamp();
        conn.execute(
            "INSERT INTO store_entries (sha256, size_bytes, created_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(sha256) DO UPDATE
             SET size_bytes = excluded.size_bytes",
            params![sha256, size_bytes, created_at],
        )?;
        Ok(())
    }

    pub fn add_store_ref(
        &self,
        conn: &Connection,
        sha256: &str,
        package: &str,
        version: &str,
    ) -> Result<()> {
        let installed_at = now_timestamp();
        conn.execute(
            "INSERT OR REPLACE INTO store_refs (sha256, package, version, installed_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![sha256, package, version, installed_at],
        )?;
        Ok(())
    }

    pub fn remove_store_ref(
        &self,
        conn: &Connection,
        sha256: &str,
        package: &str,
        version: &str,
    ) -> Result<()> {
        conn.execute(
            "DELETE FROM store_refs WHERE sha256 = ?1 AND package = ?2 AND version = ?3",
            params![sha256, package, version],
        )?;
        Ok(())
    }

    /// List all store entries that have no references (orphaned)
    pub fn list_orphaned_store_entries(&self, conn: &Connection) -> Result<Vec<StoreEntryInfo>> {
        let mut stmt = conn.prepare(
            "SELECT se.sha256, se.size_bytes, se.created_at
             FROM store_entries se
             LEFT JOIN store_refs sr ON se.sha256 = sr.sha256
             WHERE sr.sha256 IS NULL
             ORDER BY se.created_at",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(StoreEntryInfo {
                sha256: row.get(0)?,
                size_bytes: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in rows {
            entries.push(entry?);
        }
        Ok(entries)
    }

    /// Remove a store entry from the database
    pub fn delete_store_entry(&self, conn: &Connection, sha256: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM store_entries WHERE sha256 = ?1",
            params![sha256],
        )?;
        Ok(())
    }

    fn configure(conn: &Connection) -> Result<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }

    fn migrate(conn: &Connection) -> Result<()> {
        let mut version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version < 1 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS api_cache (
                    url TEXT PRIMARY KEY,
                    etag TEXT,
                    last_modified TEXT,
                    cached_at INTEGER NOT NULL
                );",
            )?;
            conn.pragma_update(None, "user_version", 1)?;
            version = 1;
        }

        if version < 2 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS blob_cache (
                    sha256 TEXT PRIMARY KEY,
                    name TEXT,
                    version TEXT,
                    tag TEXT,
                    size_bytes INTEGER NOT NULL,
                    created_at INTEGER NOT NULL
                );",
            )?;
            conn.pragma_update(None, "user_version", 2)?;
            version = 2;
        }

        if version < 3 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS store_entries (
                    sha256 TEXT PRIMARY KEY,
                    size_bytes INTEGER NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS store_refs (
                    sha256 TEXT NOT NULL,
                    package TEXT NOT NULL,
                    version TEXT NOT NULL,
                    installed_at INTEGER NOT NULL,
                    PRIMARY KEY (sha256, package, version)
                );
                CREATE INDEX IF NOT EXISTS store_refs_sha_idx ON store_refs(sha256);",
            )?;
            conn.pragma_update(None, "user_version", 3)?;
            version = 3;
        }

        if version != SCHEMA_VERSION {
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }

        Ok(())
    }
}

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_db() -> (TempDir, Database) {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        paths.init().unwrap();
        let db = Database::new(paths);
        (temp, db)
    }

    #[test]
    fn test_list_orphaned_store_entries_empty() {
        let (_temp, db) = setup_db();
        let conn = db.connect().unwrap();

        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert!(orphans.is_empty());
    }

    #[test]
    fn test_list_orphaned_store_entries_with_refs() {
        let (_temp, db) = setup_db();
        let conn = db.connect().unwrap();

        // Add a store entry with a reference - should not be orphaned
        db.upsert_store_entry(&conn, "sha256_referenced", 1000)
            .unwrap();
        db.add_store_ref(&conn, "sha256_referenced", "jq", "1.7.1")
            .unwrap();

        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert!(orphans.is_empty());
    }

    #[test]
    fn test_list_orphaned_store_entries_finds_orphans() {
        let (_temp, db) = setup_db();
        let conn = db.connect().unwrap();

        // Add an orphaned store entry (no refs)
        db.upsert_store_entry(&conn, "sha256_orphan", 2000).unwrap();

        // Add a referenced entry
        db.upsert_store_entry(&conn, "sha256_referenced", 1000)
            .unwrap();
        db.add_store_ref(&conn, "sha256_referenced", "jq", "1.7.1")
            .unwrap();

        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].sha256, "sha256_orphan");
        assert_eq!(orphans[0].size_bytes, 2000);
    }

    #[test]
    fn test_delete_store_entry() {
        let (_temp, db) = setup_db();
        let conn = db.connect().unwrap();

        // Add an entry
        db.upsert_store_entry(&conn, "sha256_to_delete", 1000)
            .unwrap();

        // Verify it exists (would be orphaned)
        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert_eq!(orphans.len(), 1);

        // Delete it
        db.delete_store_entry(&conn, "sha256_to_delete").unwrap();

        // Verify it's gone
        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert!(orphans.is_empty());
    }

    #[test]
    fn test_entry_becomes_orphaned_after_ref_removed() {
        let (_temp, db) = setup_db();
        let conn = db.connect().unwrap();

        // Add a store entry with a reference
        db.upsert_store_entry(&conn, "sha256_will_orphan", 1500)
            .unwrap();
        db.add_store_ref(&conn, "sha256_will_orphan", "node", "22.0.0")
            .unwrap();

        // Should not be orphaned yet
        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert!(orphans.is_empty());

        // Remove the reference
        db.remove_store_ref(&conn, "sha256_will_orphan", "node", "22.0.0")
            .unwrap();

        // Now should be orphaned
        let orphans = db.list_orphaned_store_entries(&conn).unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].sha256, "sha256_will_orphan");
    }
}
