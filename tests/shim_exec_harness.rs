//! Shim and hidden `crew exec` scenarios for the executable-spec harness.
//!
mod support;

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
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

impl Harness {
    fn new() -> Self {
        let temp = TempDir::new().expect("create temp root");
        let home = temp.path().join("home");
        let coldbrew_home = temp.path().join("coldbrew-home");
        let project = temp.path().join("project");
        std::fs::create_dir_all(&home).expect("create temp home");
        std::fs::create_dir_all(&coldbrew_home).expect("create coldbrew home");
        std::fs::create_dir_all(&project).expect("create project dir");

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
    let combined = format!("{}\n{}", output.stdout, output.stderr);
    assert!(
        combined.contains(needle),
        "expected output to contain {:?}\noutput:\n{}",
        needle,
        combined
    );
}

#[test]
#[ignore = "requires macOS fixture bottles"]
fn generated_shim_calls_hidden_exec() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "hello"]));

    let shim = std::fs::read_to_string(harness.shim("hello")).expect("read hello shim");
    assert!(shim.contains("# Coldbrew shim"));
    assert!(
        shim.contains("exec crew exec hello hello")
            || shim.contains("exec crew exec 'hello' 'hello'")
    );
}

#[test]
#[ignore = "requires macOS fixture bottles"]
fn hidden_exec_forwards_arguments_to_real_binary() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "hello"]));
    assert_contains(
        &harness.run(["exec", "hello", "hello", "--", "arg1", "arg2"]),
        "arg1",
    );
}

#[test]
#[ignore = "requires macOS fixture bottles"]
fn version_file_takes_precedence_over_global_default_and_latest() {
    let harness = Harness::new();

    for (version, fixture) in [("1.0.0", "multi@1"), ("2.0.0", "multi@2")] {
        let bin = harness.cellar_package("multi", version).join("bin");
        std::fs::create_dir_all(&bin).expect("create multi-version fixture");
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/bottle-payloads")
                .join(fixture)
                .join("bin/multi"),
            bin.join("multi"),
        )
        .expect("copy multi-version fixture binary");
    }
    assert_success(&harness.run(["default", "multi@1.0.0"]));
    std::fs::write(harness.project.join(".tool-versions"), "multi 2.0.0\n")
        .expect("write version file");

    assert_contains(&harness.run(["exec", "multi", "multi"]), "multi fixture 2");
}

#[test]
#[ignore = "requires macOS fixture bottles"]
fn exec_sets_dependency_library_paths() {
    let harness = Harness::new();

    assert_success(&harness.run(["install", "uses-dep"]));

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
