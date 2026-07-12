//! Shim and hidden `crew exec` scenarios for the executable-spec harness.
//!
//! These are ignored until fixture install support can create real cellar state.

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
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

    fn shim(&self, binary: &str) -> PathBuf {
        self.coldbrew_home.join("bin").join(binary)
    }

    fn cellar_package(&self, name: &str, version: &str) -> PathBuf {
        self.coldbrew_home.join("cellar").join(name).join(version)
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

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn generated_shim_calls_hidden_exec() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "hello"]));

    let shim = std::fs::read_to_string(harness.shim("hello")).expect("read hello shim");
    assert!(shim.contains("# Coldbrew shim"));
    assert!(shim.contains("exec crew exec hello hello"));
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn hidden_exec_forwards_arguments_to_real_binary() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "hello"]));
    assert_contains(&harness.run(["exec", "hello", "hello", "--", "arg1", "arg2"]), "arg1");
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn version_file_takes_precedence_over_global_default_and_latest() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "multi@1"]));
    assert_success(&harness.run(["install", "multi@2"]));
    assert_success(&harness.run(["default", "multi@1"]));
    std::fs::write(harness.project.join(".tool-versions"), "multi 2.0.0\n").expect("write version file");

    assert_contains(&harness.run(["exec", "multi", "multi"]), "multi fixture 2");
}

#[test]
#[ignore = "requires PR011 fixture registry server wiring"]
fn exec_sets_dependency_library_paths() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "uses-dep"]));
    std::fs::write(
        harness
            .cellar_package("uses-dep", "1.0.0")
            .join("bin/uses-dep"),
        "#!/bin/sh\nprintf '%s\\n' \"$DYLD_LIBRARY_PATH\"\n",
    )
    .expect("write dependency-path fixture");

    let output = harness.run(["exec", "uses-dep", "uses-dep"]);
    assert_success(&output);
    assert_contains(
        &output,
        &harness
            .cellar_package("dep", "1.0.0")
            .join("lib")
            .to_string_lossy(),
    );
}
