//! Cross-binary compatibility scenarios for the executable-spec harness.
//!
//! These scenarios require both Rust and Swift binaries plus fixture install
//! wiring, so they are ignored until the install implementation PRs can satisfy
//! them end to end.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use rusqlite::Connection;
use tempfile::TempDir;

struct CrossBinaryHarness {
    rust_bin: PathBuf,
    swift_bin: PathBuf,
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

impl Output {
    fn combined(&self) -> String {
        [self.stdout.as_str(), self.stderr.as_str()].join("\n")
    }
}

impl CrossBinaryHarness {
    fn new() -> Self {
        let temp = TempDir::new().expect("create temp root");
        let home = temp.path().join("home");
        let coldbrew_home = temp.path().join("coldbrew-home");
        let project = temp.path().join("project");
        std::fs::create_dir_all(&home).expect("create temp home");
        std::fs::create_dir_all(&coldbrew_home).expect("create coldbrew home");
        std::fs::create_dir_all(&project).expect("create project dir");

        Self {
            rust_bin: rust_crew_bin(),
            swift_bin: swift_crew_bin(),
            temp,
            home,
            coldbrew_home,
            project,
        }
    }

    fn run_rust<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.run(&self.rust_bin, args)
    }

    fn run_swift<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.run(&self.swift_bin, args)
    }

    fn run<I, S>(&self, bin: &Path, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(bin)
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

    fn db_file(&self) -> PathBuf {
        self.coldbrew_home.join("db").join("coldbrew.sqlite3")
    }
}

fn rust_crew_bin() -> PathBuf {
    env::var_os("RUST_CREW_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| assert_cmd::cargo::cargo_bin("crew"))
}

fn swift_crew_bin() -> PathBuf {
    env::var_os("SWIFT_CREW_BIN")
        .map(PathBuf::from)
        .expect("set SWIFT_CREW_BIN to the Swift crew executable")
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
        combined.contains(needle),
        "expected output to contain {:?}\noutput:\n{}",
        needle,
        combined
    );
}

fn assert_schema_fingerprint(db_file: &Path) {
    assert!(db_file.exists(), "expected database at {}", db_file.display());
    let conn = Connection::open(db_file).expect("open coldbrew database");
    let user_version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 3);

    let mut stmt = conn
        .prepare(
            "SELECT name, sql
             FROM sqlite_master
             WHERE type IN ('table', 'index') AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .expect("prepare schema fingerprint query");
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))
        .expect("query schema fingerprint");
    let fingerprint = rows
        .map(|row| {
            let (name, sql) = row.expect("read schema row");
            format!("{}:{}", name, sql.unwrap_or_default())
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(fingerprint.contains("api_cache:CREATE TABLE api_cache"));
    assert!(fingerprint.contains("blob_cache:CREATE TABLE blob_cache"));
    assert!(fingerprint.contains("store_entries:CREATE TABLE store_entries"));
    assert!(fingerprint.contains("store_refs:CREATE TABLE store_refs"));
    assert!(fingerprint.contains("store_refs_sha_idx:CREATE INDEX store_refs_sha_idx"));
}

fn assert_store_ref(db_file: &Path, package: &str, version: &str) {
    let conn = Connection::open(db_file).expect("open coldbrew database");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM store_refs WHERE package = ?1 AND version = ?2",
            (package, version),
            |row| row.get(0),
        )
        .expect("count store refs");
    assert_eq!(count, 1, "expected one store ref for {package} {version}");
}

#[test]
#[ignore = "requires Swift binary and fixture install wiring"]
fn rust_install_swift_list_and_uninstall() {
    let harness = CrossBinaryHarness::new();

    assert_success(&harness.run_rust(["install", "hello"]));
    assert_schema_fingerprint(&harness.db_file());
    assert_store_ref(&harness.db_file(), "hello", "1.0.0");

    assert_contains(&harness.run_swift(["list"]), "hello");
    assert_success(&harness.run_swift(["uninstall", "hello"]));
    assert_contains(&harness.run_rust(["list"]), "No packages installed");

    assert!(harness.temp.path().exists());
}

#[test]
#[ignore = "requires Swift binary and fixture install wiring"]
fn swift_install_rust_list_and_uninstall() {
    let harness = CrossBinaryHarness::new();

    assert_success(&harness.run_swift(["install", "uses-dep"]));
    assert_schema_fingerprint(&harness.db_file());
    assert_store_ref(&harness.db_file(), "dep", "1.0.0");
    assert_store_ref(&harness.db_file(), "uses-dep", "1.0.0");

    assert_contains(&harness.run_rust(["list"]), "uses-dep");
    assert_success(&harness.run_rust(["uninstall", "uses-dep", "--with-deps"]));
    assert_contains(&harness.run_swift(["list"]), "No packages installed");

    assert!(harness.temp.path().exists());
}
