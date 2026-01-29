//! Version parsing and comparison

use crate::error::{ColdbrewError, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// A parsed version with comparison support
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Version {
    /// Original version string
    original: String,
    /// Parsed components for comparison
    components: Vec<VersionComponent>,
}

/// A single component of a version
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
enum VersionComponent {
    Numeric(u64),
    Alpha(String),
    Separator,
}

impl Version {
    /// Parse a version string
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(ColdbrewError::InvalidVersion(s.to_string()));
        }

        let components = Self::parse_components(s);

        Ok(Self {
            original: s.to_string(),
            components,
        })
    }

    fn parse_components(s: &str) -> Vec<VersionComponent> {
        let mut components = Vec::new();
        let mut current = String::new();
        let mut in_numeric = false;

        for c in s.chars() {
            if c == '.' || c == '-' || c == '_' || c == '+' {
                if !current.is_empty() {
                    components.push(Self::make_component(&current, in_numeric));
                    current.clear();
                }
                components.push(VersionComponent::Separator);
                in_numeric = false;
            } else if c.is_ascii_digit() {
                if !in_numeric && !current.is_empty() {
                    components.push(Self::make_component(&current, false));
                    current.clear();
                }
                in_numeric = true;
                current.push(c);
            } else {
                if in_numeric && !current.is_empty() {
                    components.push(Self::make_component(&current, true));
                    current.clear();
                }
                in_numeric = false;
                current.push(c);
            }
        }

        if !current.is_empty() {
            components.push(Self::make_component(&current, in_numeric));
        }

        components
    }

    fn make_component(s: &str, is_numeric: bool) -> VersionComponent {
        if is_numeric {
            if let Ok(n) = s.parse() {
                return VersionComponent::Numeric(n);
            }
        }
        VersionComponent::Alpha(s.to_lowercase())
    }

    /// Get the original version string
    pub fn as_str(&self) -> &str {
        &self.original
    }

    /// Check if this is a pre-release version (contains alpha, beta, rc, etc.)
    pub fn is_prerelease(&self) -> bool {
        self.components.iter().any(|c| {
            matches!(c, VersionComponent::Alpha(s) if
                s.contains("alpha") ||
                s.contains("beta") ||
                s.contains("rc") ||
                s.contains("pre") ||
                s.contains("dev"))
        })
    }

    /// Get the major version number (first numeric component)
    pub fn major(&self) -> Option<u64> {
        self.components.iter().find_map(|c| {
            if let VersionComponent::Numeric(n) = c {
                Some(*n)
            } else {
                None
            }
        })
    }

    /// Get the minor version number (second numeric component)
    pub fn minor(&self) -> Option<u64> {
        self.components
            .iter()
            .filter_map(|c| {
                if let VersionComponent::Numeric(n) = c {
                    Some(*n)
                } else {
                    None
                }
            })
            .nth(1)
    }

    /// Get the patch version number (third numeric component)
    pub fn patch(&self) -> Option<u64> {
        self.components
            .iter()
            .filter_map(|c| {
                if let VersionComponent::Numeric(n) = c {
                    Some(*n)
                } else {
                    None
                }
            })
            .nth(2)
    }
}

impl FromStr for Version {
    type Err = ColdbrewError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut self_iter = self
            .components
            .iter()
            .filter(|c| !matches!(c, VersionComponent::Separator));
        let mut other_iter = other
            .components
            .iter()
            .filter(|c| !matches!(c, VersionComponent::Separator));

        loop {
            match (self_iter.next(), other_iter.next()) {
                (Some(a), Some(b)) => {
                    let ord = compare_components(a, b);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (None, None) => return Ordering::Equal,
            }
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn compare_components(a: &VersionComponent, b: &VersionComponent) -> Ordering {
    match (a, b) {
        (VersionComponent::Numeric(x), VersionComponent::Numeric(y)) => x.cmp(y),
        (VersionComponent::Numeric(_), VersionComponent::Alpha(_)) => Ordering::Greater,
        (VersionComponent::Alpha(_), VersionComponent::Numeric(_)) => Ordering::Less,
        (VersionComponent::Alpha(x), VersionComponent::Alpha(y)) => x.cmp(y),
        (VersionComponent::Separator, VersionComponent::Separator) => Ordering::Equal,
        (VersionComponent::Separator, _) => Ordering::Less,
        (_, VersionComponent::Separator) => Ordering::Greater,
    }
}

/// Parse a package specification like "jq@1.7" into (name, version)
pub fn parse_package_spec(spec: &str) -> (String, Option<String>) {
    if let Some((name, version)) = spec.split_once('@') {
        (name.to_string(), Some(version.to_string()))
    } else {
        (spec.to_string(), None)
    }
}

/// Check if a version matches a version constraint
pub fn version_matches(version: &Version, constraint: &str) -> bool {
    // Handle exact match
    if version.as_str() == constraint {
        return true;
    }

    // Handle major version match (e.g., "22" matches "22.0.0")
    if let Ok(constraint_num) = constraint.parse::<u64>() {
        if let Some(major) = version.major() {
            return major == constraint_num;
        }
    }

    // Handle major.minor match (e.g., "22.1" matches "22.1.5")
    let dot_count = constraint.chars().filter(|c| *c == '.').count();
    if dot_count == 1 {
        let parts: Vec<&str> = constraint.split('.').collect();
        if parts.len() == 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                return version.major() == Some(major) && version.minor() == Some(minor);
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.7.1").unwrap();
        assert_eq!(v.as_str(), "1.7.1");
        assert_eq!(v.major(), Some(1));
        assert_eq!(v.minor(), Some(7));
        assert_eq!(v.patch(), Some(1));
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.7.1").unwrap();
        let v2 = Version::parse("1.7.2").unwrap();
        let v3 = Version::parse("1.8.0").unwrap();
        let v4 = Version::parse("2.0.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
    }

    #[test]
    fn test_version_prerelease() {
        let v1 = Version::parse("1.0.0-alpha").unwrap();
        let v2 = Version::parse("1.0.0-beta.1").unwrap();
        let v3 = Version::parse("1.0.0").unwrap();

        assert!(v1.is_prerelease());
        assert!(v2.is_prerelease());
        assert!(!v3.is_prerelease());
    }

    #[test]
    fn test_version_with_letters() {
        let v1 = Version::parse("22.11.0").unwrap();
        let v2 = Version::parse("22.9.0").unwrap();

        assert!(v1 > v2);
    }

    #[test]
    fn test_parse_package_spec() {
        let (name, version) = parse_package_spec("jq@1.7");
        assert_eq!(name, "jq");
        assert_eq!(version, Some("1.7".to_string()));

        let (name, version) = parse_package_spec("jq");
        assert_eq!(name, "jq");
        assert_eq!(version, None);

        let (name, version) = parse_package_spec("node@22");
        assert_eq!(name, "node");
        assert_eq!(version, Some("22".to_string()));
    }

    #[test]
    fn test_version_matches() {
        let v = Version::parse("22.1.5").unwrap();
        assert!(version_matches(&v, "22.1.5"));
        assert!(version_matches(&v, "22"));
    }
}
