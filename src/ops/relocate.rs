//! Bottle relocation for macOS Mach-O binaries

use crate::cli::output::Output;
use crate::core::platform::{Os, Platform};
use crate::error::{ColdbrewError, Result};
use crate::storage::Paths;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

const HOMEBREW_CELLAR_PLACEHOLDER: &str = "@@HOMEBREW_CELLAR@@";
const HOMEBREW_PREFIX_PLACEHOLDER: &str = "@@HOMEBREW_PREFIX@@";

#[derive(Debug, Default)]
pub struct RelocationSummary {
    pub scanned_files: usize,
    pub mach_o_files: usize,
    pub relocated_files: usize,
    pub unhandled_placeholders: usize,
}

struct Replacement {
    placeholder: &'static str,
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadCommandKind {
    Rpath,
    IdDylib,
    LoadDylib,
}

#[derive(Debug, Clone)]
struct LoadCommandPath {
    kind: LoadCommandKind,
    value: String,
}

struct RelocateOutcome {
    relocated: bool,
    unhandled_placeholders: bool,
}

pub fn relocate_bottle(
    install_path: &Path,
    paths: &Paths,
    platform: &Platform,
    output: &Output,
) -> Result<RelocationSummary> {
    let mut summary = RelocationSummary::default();

    if platform.os != Os::MacOS {
        return Ok(summary);
    }

    let replacements = vec![
        Replacement {
            placeholder: HOMEBREW_CELLAR_PLACEHOLDER,
            value: paths.cellar_dir().to_string_lossy().to_string(),
        },
        Replacement {
            placeholder: HOMEBREW_PREFIX_PLACEHOLDER,
            value: paths.root().to_string_lossy().to_string(),
        },
    ];

    let mut unhandled_paths = Vec::new();

    for entry in WalkDir::new(install_path).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        summary.scanned_files += 1;

        let path = entry.path();
        if !is_macho_file(path)? {
            continue;
        }
        summary.mach_o_files += 1;

        if !file_contains_placeholders(path)? {
            continue;
        }

        let outcome = relocate_macho_file(path, &replacements)?;
        if outcome.relocated {
            summary.relocated_files += 1;
        }
        if outcome.unhandled_placeholders {
            summary.unhandled_placeholders += 1;
            unhandled_paths.push(path.to_path_buf());
        }
    }

    if !unhandled_paths.is_empty() {
        output.warning(&format!(
            "Found {} Mach-O files with Homebrew placeholders that could not be relocated",
            unhandled_paths.len()
        ));
        for path in unhandled_paths {
            output.debug(&format!("Unrelocated placeholder in {}", path.display()));
        }
    }

    Ok(summary)
}

pub fn codesign_macho_tree(
    install_path: &Path,
    platform: &Platform,
    output: &Output,
) -> Result<usize> {
    if platform.os != Os::MacOS {
        return Ok(0);
    }

    let mut signed = 0;

    for entry in WalkDir::new(install_path).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if !is_macho_file(path)? {
            continue;
        }

        codesign_file(path)?;
        signed += 1;
    }

    if signed > 0 {
        output.debug(&format!("Codesigned {} Mach-O files", signed));
    }

    Ok(signed)
}

fn relocate_macho_file(path: &Path, replacements: &[Replacement]) -> Result<RelocateOutcome> {
    let load_commands = otool_load_commands(path)?;
    let mut rpath_changes = Vec::new();
    let mut dylib_changes = Vec::new();
    let mut id_change: Option<String> = None;

    for command in load_commands {
        if let Some(replaced) = replace_placeholders(&command.value, replacements) {
            match command.kind {
                LoadCommandKind::Rpath => rpath_changes.push((command.value, replaced)),
                LoadCommandKind::IdDylib => id_change = Some(replaced),
                LoadCommandKind::LoadDylib => dylib_changes.push((command.value, replaced)),
            }
        }
    }

    if rpath_changes.is_empty() && dylib_changes.is_empty() && id_change.is_none() {
        return Ok(RelocateOutcome {
            relocated: false,
            unhandled_placeholders: true,
        });
    }

    let mut args: Vec<String> = Vec::new();
    for (old, new) in rpath_changes {
        args.push("-rpath".to_string());
        args.push(old);
        args.push(new);
    }
    if let Some(id) = id_change {
        args.push("-id".to_string());
        args.push(id);
    }
    for (old, new) in dylib_changes {
        args.push("-change".to_string());
        args.push(old);
        args.push(new);
    }
    args.push(path.to_string_lossy().to_string());

    run_tool("install_name_tool", &args)?;

    Ok(RelocateOutcome {
        relocated: true,
        unhandled_placeholders: false,
    })
}

fn otool_load_commands(path: &Path) -> Result<Vec<LoadCommandPath>> {
    let args = vec!["-l".to_string(), path.to_string_lossy().to_string()];
    let output = run_tool("otool", &args)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut commands = Vec::new();
    let mut current_cmd: Option<String> = None;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("cmd ") {
            current_cmd = Some(rest.trim().to_string());
            continue;
        }

        let Some(cmd) = current_cmd.as_deref() else {
            continue;
        };

        if cmd == "LC_RPATH" {
            if let Some(path_value) = parse_load_command_value(trimmed, "path ") {
                commands.push(LoadCommandPath {
                    kind: LoadCommandKind::Rpath,
                    value: path_value,
                });
            }
            continue;
        }

        if cmd.starts_with("LC_") && cmd.contains("DYLIB") {
            if let Some(name_value) = parse_load_command_value(trimmed, "name ") {
                let kind = if cmd == "LC_ID_DYLIB" {
                    LoadCommandKind::IdDylib
                } else {
                    LoadCommandKind::LoadDylib
                };
                commands.push(LoadCommandPath {
                    kind,
                    value: name_value,
                });
            }
        }
    }

    Ok(commands)
}

fn parse_load_command_value(line: &str, prefix: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with(prefix) {
        return None;
    }

    let value = trimmed.strip_prefix(prefix)?;
    let end = value.find(" (offset ").unwrap_or(value.len());
    Some(value[..end].trim().to_string())
}

fn replace_placeholders(value: &str, replacements: &[Replacement]) -> Option<String> {
    let mut updated = value.to_string();
    for replacement in replacements {
        if updated.contains(replacement.placeholder) {
            updated = updated.replace(replacement.placeholder, &replacement.value);
        }
    }

    if updated != value {
        Some(updated)
    } else {
        None
    }
}

fn file_contains_placeholders(path: &Path) -> Result<bool> {
    let placeholders = [
        HOMEBREW_CELLAR_PLACEHOLDER.as_bytes(),
        HOMEBREW_PREFIX_PLACEHOLDER.as_bytes(),
    ];
    let max_len = placeholders
        .iter()
        .map(|placeholder| placeholder.len())
        .max()
        .unwrap_or(0);

    let mut file = File::open(path)?;
    let mut buffer = [0u8; 8192];
    let mut carry: Vec<u8> = Vec::new();

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        let mut chunk = Vec::with_capacity(carry.len() + read);
        chunk.extend_from_slice(&carry);
        chunk.extend_from_slice(&buffer[..read]);

        if placeholders
            .iter()
            .any(|placeholder| contains_subslice(&chunk, placeholder))
        {
            return Ok(true);
        }

        if max_len > 1 {
            let start = chunk.len().saturating_sub(max_len - 1);
            carry = chunk[start..].to_vec();
        } else {
            carry.clear();
        }
    }

    Ok(false)
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn is_macho_file(path: &Path) -> Result<bool> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 4];
    let read = file.read(&mut magic)?;
    if read < 4 {
        return Ok(false);
    }

    let be = u32::from_be_bytes(magic);
    let le = u32::from_le_bytes(magic);

    const MH_MAGIC: u32 = 0xfeedface;
    const MH_MAGIC_64: u32 = 0xfeedfacf;
    const FAT_MAGIC: u32 = 0xcafebabe;
    const FAT_MAGIC_64: u32 = 0xcafebabf;

    Ok(
        matches!(be, MH_MAGIC | MH_MAGIC_64 | FAT_MAGIC | FAT_MAGIC_64)
            || matches!(le, MH_MAGIC | MH_MAGIC_64 | FAT_MAGIC | FAT_MAGIC_64),
    )
}

fn codesign_file(path: &Path) -> Result<()> {
    let args = vec![
        "--sign".to_string(),
        "-".to_string(),
        "--force".to_string(),
        "--timestamp=none".to_string(),
        path.to_string_lossy().to_string(),
    ];
    run_tool("codesign", &args)?;
    Ok(())
}

fn run_tool(tool: &str, args: &[String]) -> Result<std::process::Output> {
    let output = Command::new(tool)
        .args(args)
        .output()
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => ColdbrewError::Other(format!(
                "Required tool '{}' was not found. Install Xcode Command Line Tools with `xcode-select --install`.",
                tool
            )),
            _ => ColdbrewError::Other(format!("Failed to run '{}': {}", tool, err)),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ColdbrewError::Other(format!(
            "'{}' failed: {}",
            tool,
            stderr.trim()
        )));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_placeholders() {
        let replacements = vec![
            Replacement {
                placeholder: HOMEBREW_CELLAR_PLACEHOLDER,
                value: "/tmp/cellar".to_string(),
            },
            Replacement {
                placeholder: HOMEBREW_PREFIX_PLACEHOLDER,
                value: "/tmp".to_string(),
            },
        ];

        let value = "@@HOMEBREW_CELLAR@@/jq/1.7.1";
        let updated = replace_placeholders(value, &replacements).unwrap();
        assert_eq!(updated, "/tmp/cellar/jq/1.7.1");
    }

    #[test]
    fn test_contains_subslice() {
        assert!(contains_subslice(b"abcde", b"bcd"));
        assert!(!contains_subslice(b"abcde", b"bd"));
    }
}
