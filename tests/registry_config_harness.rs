//! Fixture-backed registry and config workflows for any `CREW_BIN` implementation.

mod support;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::ExitStatus;

use serde_json::Value;
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
        let coldbrew_home = home.join(".coldbrew");
        let project = temp.path().join("project");
        fs::create_dir_all(&home).expect("create temp home");
        fs::create_dir_all(&coldbrew_home).expect("create coldbrew home");
        fs::create_dir_all(&project).expect("create project");

        Self {
            bin: crew_bin(),
            fixture: FixtureRegistry::start(),
            _temp: temp,
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
            stdout: String::from_utf8_lossy(&output.stdout).replace('\r', ""),
            stderr: String::from_utf8_lossy(&output.stderr).replace('\r', ""),
        }
    }

    fn update(&self) {
        assert_success(&self.run(["update"]));
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

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure\nstdout:\n{}\nstderr:\n{}",
        output.stdout,
        output.stderr
    );
}

fn assert_contains(output: &Output, needle: &str) {
    let combined = output.combined();
    assert!(
        combined.contains(needle),
        "expected output to contain {needle:?}\noutput:\n{combined}"
    );
}

#[test]
fn fixture_update_search_and_info_text_and_json() {
    let harness = Harness::new();

    harness.update();
    let index: Value = serde_json::from_slice(
        &fs::read(harness.coldbrew_home.join("index/formula.json")).expect("read formula index"),
    )
    .expect("parse formula index");
    assert_eq!(index.as_array().map(Vec::len), Some(5));

    let search = harness.run(["search", "hello", "--extended"]);
    assert_success(&search);
    assert_contains(&search, "hello");
    assert_contains(&search, "Fixture package with one executable");

    let text = harness.run(["info", "hello"]);
    assert_success(&text);
    for expected in [
        "hello",
        "1.0.0",
        "Fixture package with one executable",
        "https://example.test/hello",
    ] {
        assert_contains(&text, expected);
    }

    let json = harness.run(["info", "hello", "--format", "json"]);
    assert_success(&json);
    let formula: Value = serde_json::from_str(json.stdout.trim()).expect("parse info JSON");
    assert_eq!(formula["name"], "hello");
    assert_eq!(formula["versions"]["stable"], "1.0.0");
}

#[test]
fn init_lock_and_install_from_lockfile() {
    let harness = Harness::new();
    harness.update();

    assert_success(&harness.run(["init"]));
    let config = harness.project.join("coldbrew.toml");
    assert!(config.exists(), "init must create coldbrew.toml");
    fs::write(&config, "[packages]\nhello = \"1.0.0\"\n\n[dev_packages]\n")
        .expect("write fixture project config");

    assert_success(&harness.run(["lock"]));
    let lock = fs::read_to_string(harness.project.join("coldbrew.lock")).expect("read lockfile");
    for expected in ["[packages.hello]", "version = \"1.0.0\"", "config_hash"] {
        assert!(
            lock.contains(expected),
            "lockfile must contain {expected:?}\n{lock}"
        );
    }

    assert_success(&harness.run(["install", "--lock"]));
    assert!(
        harness.coldbrew_home.join("cellar/hello/1.0.0").exists(),
        "install --lock must install the locked package"
    );
}

#[test]
fn doctor_and_zsh_completions_are_usable() {
    let harness = Harness::new();
    harness.update();

    let doctor = harness.run(["doctor"]);
    assert_success(&doctor);
    assert_contains(&doctor, "Coldbrew");

    let completions = harness.run(["completions", "zsh"]);
    assert_success(&completions);
    assert_contains(&completions, "#compdef crew");
}

#[test]
fn tap_lists_empty_state_and_rejects_invalid_name() {
    let harness = Harness::new();

    let empty = harness.run(["tap"]);
    assert_success(&empty);
    assert_contains(&empty, "No taps installed");

    let invalid = harness.run(["tap", "invalid"]);
    assert_failure(&invalid);
    let message = invalid.combined().to_lowercase();
    assert!(
        message.contains("invalid tap"),
        "unexpected error:\n{message}"
    );
    assert!(
        message.contains("user/repo"),
        "unexpected error:\n{message}"
    );
}
