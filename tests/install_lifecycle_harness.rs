//! Install lifecycle scenarios for the executable-spec harness.
//!
//! These are intentionally ignored until PR011 fixture serving is wired into the harness.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use tempfile::TempDir;

struct Harness {
    bin: PathBuf,
    temp: TempDir,
    home: PathBuf,
    coldbrew_home: PathBuf,
    project: PathBuf,
}

struct Output {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl Harness {
    fn new() -> Self {
        let temp = TempDir::new().expect("create temp root");
        let home = temp.path().join("home");
        let coldbrew_home = temp.path().join("coldbrew-home");
        let project = temp.path().join("project");
        std::fs::create_dir_all(&home).expect("create temp home");
        std::fs::create_dir_all(&coldbrew_home).expect("create coldbrew home");
        std::fs::create_dir_all(&project).expect("create project dir");

        Self {
            bin: crew_bin(),
            temp,
            home,
            coldbrew_home,
            project,
        }
    }

    fn run<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.bin)
            .args(args)
            .current_dir(&self.project)
            .env("COLDBREW_HOME", &self.coldbrew_home)
            .env("HOME", &self.home)
            .env("NO_COLOR", "1")
            .env("CLICOLOR", "0")
            .env("TERM", "dumb")
            .output()
            .expect("run crew");

        Output {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    fn cellar_package(&self, name: &str, version: &str) -> PathBuf {
        self.coldbrew_home.join("cellar").join(name).join(version)
    }

    fn shim(&self, binary: &str) -> PathBuf {
        self.coldbrew_home.join("bin").join(binary)
    }
}

fn crew_bin() -> PathBuf {
    env::var_os("CREW_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| assert_cmd::cargo::cargo_bin("crew"))
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
    let combined = format!("{}\n{}", output.stdout, output.stderr);
    assert!(
        combined.contains(needle),
        "expected output to contain {:?}\noutput:\n{}",
        needle,
        combined
    );
}

fn assert_path_exists(path: &Path) {
    assert!(path.exists(), "expected path to exist: {}", path.display());
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn install_single_package_creates_cellar_store_and_shim() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "hello"]));

    assert_path_exists(&harness.cellar_package("hello", "1.0.0"));
    assert_path_exists(&harness.shim("hello"));
    assert_path_exists(&harness.coldbrew_home.join("store"));
    assert!(harness.temp.path().exists());
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn install_with_dependency_installs_dependency_first() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "uses-dep"]));

    assert_path_exists(&harness.cellar_package("dep", "1.0.0"));
    assert_path_exists(&harness.cellar_package("uses-dep", "1.0.0"));
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn install_force_reinstalls_existing_package() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "hello"]));
    assert_success(&harness.run(["install", "--force", "hello"]));
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn install_skip_deps_omits_runtime_dependencies() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "--skip-deps", "uses-dep"]));

    assert!(!harness.cellar_package("dep", "1.0.0").exists());
    assert_path_exists(&harness.cellar_package("uses-dep", "1.0.0"));
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn list_and_which_report_installed_package() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "hello"]));
    assert_contains(&harness.run(["list"]), "hello");
    assert_contains(&harness.run(["which", "hello"]), "hello");
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn uninstall_removes_package_and_with_deps_removes_unused_dependencies() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "uses-dep"]));
    assert_success(&harness.run(["uninstall", "--with-deps", "uses-dep"]));

    assert!(!harness.cellar_package("uses-dep", "1.0.0").exists());
    assert!(!harness.cellar_package("dep", "1.0.0").exists());
}
