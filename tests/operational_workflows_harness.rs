//! Fixture-backed operational workflows shared by the Rust and Swift CLIs.

mod support;

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use support::FixtureRegistry;
use tempfile::TempDir;

struct Harness {
    bin: PathBuf,
    fixture: FixtureRegistry,
    _temp: TempDir,
    home: PathBuf,
    coldbrew_home: PathBuf,
    project: PathBuf,
}

struct Output {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl Output {
    fn combined(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

impl Harness {
    fn new() -> Self {
        let temp = TempDir::new().expect("create temp root");
        let home = temp.path().join("home");
        let coldbrew_home = temp.path().join("coldbrew-home");
        let project = temp.path().join("project");
        for directory in [&home, &coldbrew_home, &project] {
            std::fs::create_dir_all(directory).expect("create harness directory");
        }

        let harness = Self {
            bin: crew_bin(),
            fixture: FixtureRegistry::start(),
            _temp: temp,
            home,
            coldbrew_home,
            project,
        };
        assert_success(&harness.run(["update"]));
        harness
    }

    fn run<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = support::process::command(&self.bin)
            .args(args)
            .current_dir(&self.project)
            .env("COLDBREW_HOME", &self.coldbrew_home)
            .env("COLDBREW_FORMULA_API_BASE", self.fixture.base_url())
            .env("HOME", &self.home)
            .env("NO_COLOR", "1")
            .env("CLICOLOR", "0")
            .env("TERM", "dumb")
            .output()
            .expect("run crew");

        Output {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn install(&self, package: &str) {
        assert_success(&self.run(["install", package]));
    }

    fn shim(&self, binary: &str) -> PathBuf {
        self.coldbrew_home.join("bin").join(binary)
    }
}

fn crew_bin() -> PathBuf {
    env::var_os("CREW_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| assert_cmd::cargo::cargo_bin!("crew").to_path_buf())
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        output.stdout,
        output.stderr
    );
}

fn assert_contains(output: &Output, needle: &str) {
    let combined = output.combined();
    assert!(
        combined.to_lowercase().contains(&needle.to_lowercase()),
        "expected output to contain {needle:?}\noutput:\n{combined}"
    );
}

fn assert_exists(path: &Path, expected: bool) {
    assert_eq!(
        path.exists(),
        expected,
        "expected {} to {}exist",
        path.display(),
        if expected { "" } else { "not " }
    );
}

#[test]
fn dependents_pin_unpin_and_default_are_reported_and_persisted() {
    let harness = Harness::new();
    harness.install("uses-dep");

    let dependents = harness.run(["dependents", "dep"]);
    assert_success(&dependents);
    assert_contains(&dependents, "uses-dep");

    assert_success(&harness.run(["pin", "uses-dep"]));
    let config = harness.coldbrew_home.join("config.toml");
    assert!(toml_section(&config, "pins").contains("uses-dep"));
    assert_success(&harness.run(["unpin", "uses-dep"]));
    assert!(!toml_section(&config, "pins").contains("uses-dep"));

    assert_success(&harness.run(["default", "uses-dep@1.0.0"]));
    assert_contains(&harness.run(["default", "uses-dep"]), "1.0.0");
}

#[test]
fn unlink_link_and_which_track_the_shim() {
    let harness = Harness::new();
    harness.install("hello");
    let shim = harness.shim("hello");

    let which = harness.run(["which", "hello"]);
    assert_success(&which);
    assert_contains(&which, "hello");
    assert_exists(&shim, true);

    assert_success(&harness.run(["unlink", "hello"]));
    assert_exists(&shim, false);
    assert_success(&harness.run(["link", "hello"]));
    assert_exists(&shim, true);
    assert_success(&harness.run(["which", "hello"]));
}

#[test]
fn upgrade_yes_reports_when_everything_is_current() {
    let harness = Harness::new();
    harness.install("hello");

    let upgrade = harness.run(["upgrade", "--yes"]);
    assert_success(&upgrade);
    let combined = upgrade.combined().to_lowercase();
    assert!(
        combined.contains("no upgrades available")
            || combined.contains("all packages are up to date"),
        "unexpected upgrade output:\n{combined}"
    );
}

#[test]
fn space_dry_run_preserves_and_clean_removes_orphaned_store_entries() {
    let harness = Harness::new();
    harness.install("hello");
    let entries = std::fs::read_dir(harness.coldbrew_home.join("store"))
        .expect("read store")
        .map(|entry| entry.expect("read store entry").path())
        .collect::<Vec<_>>();
    assert!(!entries.is_empty(), "install must create a store entry");
    assert_success(&harness.run(["uninstall", "hello"]));

    let show = harness.run(["space", "show"]);
    assert_success(&show);
    assert_contains(&show, "orphaned store");

    let dry_run = harness.run(["space", "clean", "--dry-run", "--all"]);
    assert_success(&dry_run);
    assert_contains(&dry_run, "remove");
    for entry in &entries {
        assert_exists(entry, true);
    }

    assert_success(&harness.run(["space", "clean", "--all"]));
    for entry in &entries {
        assert_exists(entry, false);
    }
}

#[test]
fn tool_versions_resolves_a_fixture_package_and_unrelated_version_files_do_not() {
    let harness = Harness::new();
    harness.install("hello");

    std::fs::write(harness.project.join(".nvmrc"), "v2.0.0\n").expect("write .nvmrc");
    std::fs::write(harness.project.join(".python-version"), "2.0.0\n")
        .expect("write .python-version");
    std::fs::write(harness.project.join(".tool-versions"), "hello 1.0.0\n")
        .expect("write .tool-versions");
    assert_contains(&harness.run(["exec", "hello", "hello"]), "hello fixture");

    std::fs::remove_file(harness.project.join(".tool-versions")).expect("remove .tool-versions");
    assert_contains(&harness.run(["exec", "hello", "hello"]), "hello fixture");
}

fn toml_section(path: &Path, name: &str) -> String {
    let text = std::fs::read_to_string(path).expect("read text file");
    text.split(&format!("[{name}]"))
        .nth(1)
        .unwrap_or_default()
        .split("\n[")
        .next()
        .unwrap_or_default()
        .to_owned()
}
