use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn write_formula_index(home: &std::path::Path) {
    let index_dir = home.join(".coldbrew").join("index");
    fs::create_dir_all(&index_dir).unwrap();

    let formulas = r#"
[
  {
    "name": "jq",
    "full_name": "homebrew/core/jq",
    "versions": { "stable": "1.7.1", "head": null, "bottle": true }
  },
  {
    "name": "node@22",
    "full_name": "homebrew/core/node@22",
    "versions": { "stable": "22.1.0", "head": null, "bottle": true }
  },
  {
    "name": "ripgrep",
    "full_name": "homebrew/core/ripgrep",
    "versions": { "stable": "14.0.0", "head": null, "bottle": true }
  }
]
"#;

    fs::write(index_dir.join("formula.json"), formulas).unwrap();
}

fn write_fake_brew(bin_dir: &std::path::Path) -> std::path::PathBuf {
    fs::create_dir_all(bin_dir).unwrap();
    let brew_path = bin_dir.join("brew");
    let script = r#"#!/bin/sh
set -e

if [ "$1" = "leaves" ] && [ "$2" = "--installed-on-request" ]; then
  echo "jq"
  echo "node@22"
  echo "ripgrep"
  exit 0
fi

if [ "$1" = "info" ] && [ "$2" = "--json=v2" ] && [ "$3" = "--installed" ]; then
  cat <<'JSON'
{
  "formulae": [
    { "name": "jq", "installed": [{ "version": "1.7.1" }], "linked_keg": "1.7.1" },
    { "name": "node@22", "installed": [{ "version": "22.1.0" }], "linked_keg": "22.1.0" },
    {
      "name": "ripgrep",
      "installed": [{ "version": "13.0.0" }, { "version": "14.0.0" }],
      "linked_keg": "13.0.0"
    }
  ],
  "casks": []
}
JSON
  exit 0
fi

if [ "$1" = "list" ] && [ "$2" = "--cask" ]; then
  echo "google-chrome"
  exit 0
fi

echo "unexpected brew args: $*" >&2
exit 1
"#;

    fs::write(&brew_path, script).unwrap();
    let mut perms = fs::metadata(&brew_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&brew_path, perms).unwrap();
    brew_path
}

#[test]
fn migrate_dry_run_handles_casks_and_imports_keg_versions() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_formula_index(&home);

    let bin_dir = temp.path().join("bin");
    let brew_path = write_fake_brew(&bin_dir);

    cargo_bin_cmd!("crew")
        .args([
            "migrate",
            "--dry-run",
            "--brew",
            brew_path.to_str().unwrap(),
        ])
        .env("HOME", &home)
        .env("PATH", &bin_dir)
        .assert()
        .success()
        .stderr(predicate::str::contains("Skipping 1 Homebrew cask"))
        .stderr(predicate::str::contains("importing Homebrew keg"))
        .stderr(predicate::str::contains(
            "Migration complete: would migrate 3, 0 skipped, 0 failed",
        ));
}
