//! Core data structures and types

pub mod bottle;
pub mod dependency;
pub mod formula;
pub mod package;
pub mod platform;
pub mod version;

pub use bottle::{BottleFile, BottleSpec, CellarType};
pub use dependency::DependencyResolver;
pub use formula::Formula;
pub use package::InstalledPackage;
pub use platform::Platform;
pub use version::Version;
