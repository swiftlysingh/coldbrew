//! Storage management for packages, cache, and shims

pub mod cache;
pub mod cellar;
pub mod db;
pub mod paths;
pub mod shim;
pub mod store;

pub use cache::Cache;
pub use cellar::Cellar;
pub use db::Database;
pub use paths::Paths;
pub use shim::ShimManager;
pub use store::{Store, StoreEntry};
