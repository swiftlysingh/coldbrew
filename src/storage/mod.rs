//! Storage management for packages, cache, and shims

pub mod cache;
pub mod cellar;
pub mod paths;
pub mod shim;

pub use cache::Cache;
pub use cellar::Cellar;
pub use paths::Paths;
pub use shim::ShimManager;
