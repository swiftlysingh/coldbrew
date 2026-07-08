//! Integration harness for Coldbrew executable-spec tests.
//!
//! Run with: cargo test --test integration

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use tempfile::TempDir;

struct CrewHarness {
    bin: PathBuf,
    temp: TempDir,
    home: PathBuf,
    coldbrew_home: PathBuf,
    project: PathBuf,
}

struct CrewOutput {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl CrewOutput {
    fn combined(&self) -> String {
        [self.stdout.as_str(), self.stderr.as_str()].join("\n")
    }
}

impl CrewHarness {
    fn new() -> Self {
        let temp = TempDir::new().expect("create temp test root");
        let root = temp.path();
        let home = root.join("home");
        let coldbrew_home = root.join("coldbrew-home");
        let project = root.join("project");

        std::fs::create_dir_all(&home).expect("create temp home");
        std::fs::create_dir_all(&coldbrew_home).expect("create temp coldbrew home");
        std::fs::create_dir_all(&project).expect("create temp project");

        Self {
            bin: crew_bin(),
            temp,
            home,
            coldbrew_home,
            project,
        }
    }

    fn run<I, S>(&self, args: I) -> CrewOutput
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.bin)
            .args(args)
            .current_dir(&self.project)
            .env("COLDBREW_HOME", &self.coldbrew_home)
            // Current Rust Paths uses HOME/.coldbrew; keep tests isolated until
            // COLDBREW_HOME is wired into both implementations.
            .env("HOME", &self.home)
            .env("NO_COLOR", "1")
            .env("CLICOLOR", "0")
            .env("TERM", "dumb")
            .output()
            .expect("run crew");

        CrewOutput {
            status: output.status,
            stdout: normalize_stream(&output.stdout),
            stderr: normalize_stream(&output.stderr),
        }
    }
}

fn crew_bin() -> PathBuf {
    env::var_os("CREW_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| assert_cmd::cargo::cargo_bin("crew"))
}

fn normalize_stream(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    strip_ansi(&text)
        .replace('\r', "\n")
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !is_progress_frame(line))
        .map(normalize_dynamic_tokens)
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for code in chars.by_ref() {
                if ('@'..='~').contains(&code) {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }

    out
}

fn is_progress_frame(line: &str) -> bool {
    line.contains("ETA") || line.contains("/s)") || line.contains("elapsed")
}

fn normalize_dynamic_tokens(line: &str) -> String {
    line.split_whitespace()
        .map(|token| {
            if looks_like_duration(token) {
                "<duration>"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_duration(token: &str) -> bool {
    let token = token.trim_matches(['[', ']', '(', ')', ',']);
    token.contains(':') && token.chars().all(|ch| ch.is_ascii_digit() || ch == ':')
}

fn assert_success(output: &CrewOutput) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        output.stdout,
        output.stderr
    );
}

fn assert_failure(output: &CrewOutput) {
    assert!(
        !output.status.success(),
        "expected failure\nstdout:\n{}\nstderr:\n{}",
        output.stdout,
        output.stderr
    );
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected output to contain {:?}\noutput:\n{}",
        needle,
        haystack
    );
}

fn assert_contains_any(haystack: &str, needles: &[&str]) {
    assert!(
        needles.iter().any(|needle| haystack.contains(needle)),
        "expected output to contain one of {:?}\noutput:\n{}",
        needles,
        haystack
    );
}

#[test]
fn help_shows_cli_usage() {
    let harness = CrewHarness::new();
    let output = harness.run(["--help"]);

    assert_success(&output);
    assert_contains_any(&output.stdout, &["Usage:", "USAGE:"]);
    assert_contains_any(&output.stdout, &["Commands:", "SUBCOMMANDS:"]);
    assert_contains(&output.stdout, "install");
}

#[test]
fn no_command_prints_help() {
    let harness = CrewHarness::new();
    let output = harness.run(std::iter::empty::<&str>());

    assert_success(&output);
    assert_contains_any(&output.stdout, &["Usage:", "USAGE:"]);
    assert_contains_any(&output.stdout, &["Commands:", "SUBCOMMANDS:"]);
}

#[test]
fn invalid_command_fails() {
    let harness = CrewHarness::new();
    let output = harness.run(["definitely-not-a-command"]);

    assert_failure(&output);
    assert_contains(&output.stderr.to_lowercase(), "error");
    assert_contains(&output.stderr, "definitely-not-a-command");
}

#[test]
fn init_creates_project_config() {
    let harness = CrewHarness::new();
    let output = harness.run(["init"]);

    assert_success(&output);
    assert!(harness.project.join("coldbrew.toml").exists());

    let combined = output.combined();
    assert_contains(&combined, "Created");
    assert_contains(&combined, "coldbrew.toml");
}

#[test]
fn list_is_empty_in_fresh_home() {
    let harness = CrewHarness::new();
    let output = harness.run(["list"]);

    assert_success(&output);
    assert_contains(&output.combined(), "No packages installed");

    // Keep the temp root alive for the whole scenario and prove each scenario is isolated.
    assert!(harness.temp.path().exists());
}
