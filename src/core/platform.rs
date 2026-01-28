//! Platform detection for bottle selection

use crate::error::{ColdbrewError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Operating system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    MacOS,
    Linux,
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Os::MacOS => write!(f, "macos"),
            Os::Linux => write!(f, "linux"),
        }
    }
}

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Arm64,
    X86_64,
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arch::Arm64 => write!(f, "arm64"),
            Arch::X86_64 => write!(f, "x86_64"),
        }
    }
}

/// macOS version codenames
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacOsVersion {
    Sequoia,   // 15.x
    Sonoma,    // 14.x
    Ventura,   // 13.x
    Monterey,  // 12.x
    BigSur,    // 11.x
    Catalina,  // 10.15
    Unknown(String),
}

impl MacOsVersion {
    /// Parse macOS version from the major version number
    pub fn from_major_version(major: u32) -> Self {
        match major {
            15 => MacOsVersion::Sequoia,
            14 => MacOsVersion::Sonoma,
            13 => MacOsVersion::Ventura,
            12 => MacOsVersion::Monterey,
            11 => MacOsVersion::BigSur,
            10 => MacOsVersion::Catalina, // Technically 10.15, but we'll use major only
            _ => MacOsVersion::Unknown(format!("macos{}", major)),
        }
    }

    /// Get the bottle tag name for this macOS version
    pub fn bottle_tag(&self) -> &str {
        match self {
            MacOsVersion::Sequoia => "sequoia",
            MacOsVersion::Sonoma => "sonoma",
            MacOsVersion::Ventura => "ventura",
            MacOsVersion::Monterey => "monterey",
            MacOsVersion::BigSur => "big_sur",
            MacOsVersion::Catalina => "catalina",
            MacOsVersion::Unknown(s) => s,
        }
    }

    /// Get fallback versions in order of preference
    pub fn fallbacks(&self) -> Vec<MacOsVersion> {
        match self {
            MacOsVersion::Sequoia => vec![MacOsVersion::Sonoma, MacOsVersion::Ventura],
            MacOsVersion::Sonoma => vec![MacOsVersion::Ventura, MacOsVersion::Monterey],
            MacOsVersion::Ventura => vec![MacOsVersion::Monterey, MacOsVersion::BigSur],
            MacOsVersion::Monterey => vec![MacOsVersion::BigSur, MacOsVersion::Catalina],
            MacOsVersion::BigSur => vec![MacOsVersion::Catalina],
            MacOsVersion::Catalina => vec![],
            MacOsVersion::Unknown(_) => vec![MacOsVersion::Sonoma, MacOsVersion::Ventura],
        }
    }
}

impl fmt::Display for MacOsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.bottle_tag())
    }
}

/// Platform information for the current system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
    pub os_version: Option<MacOsVersion>,
}

impl Platform {
    /// Detect the current platform
    pub fn detect() -> Result<Self> {
        let os = Self::detect_os()?;
        let arch = Self::detect_arch()?;
        let os_version = if os == Os::MacOS {
            Some(Self::detect_macos_version()?)
        } else {
            None
        };

        Ok(Platform {
            os,
            arch,
            os_version,
        })
    }

    fn detect_os() -> Result<Os> {
        if cfg!(target_os = "macos") {
            Ok(Os::MacOS)
        } else if cfg!(target_os = "linux") {
            Ok(Os::Linux)
        } else {
            Err(ColdbrewError::UnsupportedPlatform {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
            })
        }
    }

    fn detect_arch() -> Result<Arch> {
        if cfg!(target_arch = "aarch64") {
            Ok(Arch::Arm64)
        } else if cfg!(target_arch = "x86_64") {
            Ok(Arch::X86_64)
        } else {
            Err(ColdbrewError::UnsupportedPlatform {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
            })
        }
    }

    #[cfg(target_os = "macos")]
    fn detect_macos_version() -> Result<MacOsVersion> {
        use std::process::Command;

        let output = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map_err(|e| ColdbrewError::Other(format!("Failed to detect macOS version: {}", e)))?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version_str = version_str.trim();

        // Parse major version (e.g., "14.2.1" -> 14)
        let major: u32 = version_str
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(14); // Default to Sonoma if parsing fails

        Ok(MacOsVersion::from_major_version(major))
    }

    #[cfg(not(target_os = "macos"))]
    fn detect_macos_version() -> Result<MacOsVersion> {
        Ok(MacOsVersion::Unknown("unknown".to_string()))
    }

    /// Generate the bottle tag for this platform (e.g., "arm64_sequoia")
    pub fn bottle_tag(&self) -> String {
        match self.os {
            Os::MacOS => {
                let version = self
                    .os_version
                    .as_ref()
                    .map(|v| v.bottle_tag())
                    .unwrap_or("sonoma");

                match self.arch {
                    Arch::Arm64 => format!("arm64_{}", version),
                    Arch::X86_64 => version.to_string(),
                }
            }
            Os::Linux => {
                match self.arch {
                    Arch::Arm64 => "arm64_linux".to_string(),
                    Arch::X86_64 => "x86_64_linux".to_string(),
                }
            }
        }
    }

    /// Get all bottle tags to try, in order of preference (exact match first, then fallbacks)
    pub fn bottle_tags(&self) -> Vec<String> {
        let mut tags = vec![self.bottle_tag()];

        if let Some(ref os_version) = self.os_version {
            for fallback in os_version.fallbacks() {
                let tag = match self.arch {
                    Arch::Arm64 => format!("arm64_{}", fallback.bottle_tag()),
                    Arch::X86_64 => fallback.bottle_tag().to_string(),
                };
                tags.push(tag);
            }
        }

        // Add "all" as last resort
        tags.push("all".to_string());

        tags
    }

    /// Check if a bottle tag is compatible with this platform
    pub fn is_compatible(&self, bottle_tag: &str) -> bool {
        self.bottle_tags().contains(&bottle_tag.to_string())
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.os, self.arch)?;
        if let Some(ref version) = self.os_version {
            write!(f, " ({})", version)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bottle_tag_arm64_sequoia() {
        let platform = Platform {
            os: Os::MacOS,
            arch: Arch::Arm64,
            os_version: Some(MacOsVersion::Sequoia),
        };
        assert_eq!(platform.bottle_tag(), "arm64_sequoia");
    }

    #[test]
    fn test_bottle_tag_x86_64_sonoma() {
        let platform = Platform {
            os: Os::MacOS,
            arch: Arch::X86_64,
            os_version: Some(MacOsVersion::Sonoma),
        };
        assert_eq!(platform.bottle_tag(), "sonoma");
    }

    #[test]
    fn test_bottle_tag_linux() {
        let platform = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
            os_version: None,
        };
        assert_eq!(platform.bottle_tag(), "x86_64_linux");
    }

    #[test]
    fn test_bottle_tags_fallback() {
        let platform = Platform {
            os: Os::MacOS,
            arch: Arch::Arm64,
            os_version: Some(MacOsVersion::Sequoia),
        };
        let tags = platform.bottle_tags();
        assert!(tags.contains(&"arm64_sequoia".to_string()));
        assert!(tags.contains(&"arm64_sonoma".to_string()));
        assert!(tags.contains(&"arm64_ventura".to_string()));
        assert!(tags.contains(&"all".to_string()));
    }
}
