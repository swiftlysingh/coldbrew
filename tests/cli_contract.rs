//! CLI shape contract for the Swift migration.
//!
//! These tests lock command and flag presence without exercising command behavior.

#[path = "support/process.rs"]
mod process;

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;

struct CrewOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

fn crew_bin() -> PathBuf {
    env::var_os("CREW_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| assert_cmd::cargo::cargo_bin!("crew"))
}

fn run<I, S>(args: I) -> CrewOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = process::command(crew_bin())
        .args(args)
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env("TERM", "dumb")
        .output()
        .expect("run crew");

    CrewOutput {
        status: output.status,
        stdout: normalize(&output.stdout),
        stderr: normalize(&output.stderr),
    }
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace('\r', "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_success(output: &CrewOutput) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        output.stdout,
        output.stderr
    );
}

fn assert_contains(output: &str, needle: &str) {
    assert!(
        output.contains(needle),
        "expected output to contain {:?}\noutput:\n{}",
        needle,
        output
    );
}

fn assert_contains_any(output: &str, needles: &[&str]) {
    assert!(
        needles.iter().any(|needle| output.contains(needle)),
        "expected output to contain one of {:?}\noutput:\n{}",
        needles,
        output
    );
}

fn assert_command_listed(help: &str, command: &str) {
    assert!(
        help.lines()
            .map(str::trim_start)
            .any(|line| line.starts_with(command)
                && line[command.len()..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)),
        "expected root help to list command {:?}\nhelp:\n{}",
        command,
        help
    );
}

fn assert_command_hidden(help: &str, command: &str) {
    assert!(
        !help
            .lines()
            .map(str::trim_start)
            .any(|line| line.starts_with(command)
                && line[command.len()..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)),
        "expected root help to hide command {:?}\nhelp:\n{}",
        command,
        help
    );
}

fn assert_help(args: &[&str], expected: &[&[&str]]) {
    let output = run(args.iter().copied());

    assert_success(&output);
    assert_contains_any(&output.stdout, &["Usage:", "USAGE:"]);
    for alternatives in expected {
        assert_contains_any(&output.stdout, alternatives);
    }
}

#[test]
fn root_help_lists_public_commands_and_global_flags() {
    let output = run(["--help"]);

    assert_success(&output);
    assert_contains_any(&output.stdout, &["Usage:", "USAGE:"]);
    assert_contains(&output.stdout, "--quiet");
    assert_contains(&output.stdout, "--verbose");

    for command in [
        "update",
        "search",
        "info",
        "install",
        "uninstall",
        "upgrade",
        "list",
        "which",
        "pin",
        "unpin",
        "default",
        "dependents",
        "init",
        "lock",
        "tap",
        "space",
        "link",
        "unlink",
        "doctor",
        "completions",
    ] {
        assert_command_listed(&output.stdout, command);
    }

    assert_command_hidden(&output.stdout, "exec");
}

#[test]
fn command_help_exposes_current_flags_and_arguments() {
    let cases: &[(&[&str], &[&[&str]])] = &[
        (
            &["search", "--help"],
            &[&["<QUERY>", "<query>"], &["--extended"]],
        ),
        (
            &["info", "--help"],
            &[&["<PACKAGE>", "<package>"], &["--format"]],
        ),
        (
            &["install", "--help"],
            &[
                &["[PACKAGES]...", "<packages> ..."],
                &["--lock"],
                &["--skip-deps"],
                &["--force"],
            ],
        ),
        (
            &["uninstall", "--help"],
            &[
                &["<PACKAGES>...", "<packages> ..."],
                &["--all"],
                &["--with-deps"],
            ],
        ),
        (
            &["upgrade", "--help"],
            &[&["[PACKAGES]...", "<packages> ..."], &["--yes"]],
        ),
        (&["list", "--help"], &[&["--names-only"], &["--versions"]]),
        (&["which", "--help"], &[&["<BINARY>", "<binary>"]]),
        (&["pin", "--help"], &[&["<PACKAGE>", "<package>"]]),
        (&["unpin", "--help"], &[&["<PACKAGE>", "<package>"]]),
        (&["default", "--help"], &[&["<PACKAGE>", "<package>"]]),
        (&["dependents", "--help"], &[&["<PACKAGE>", "<package>"]]),
        (&["init", "--help"], &[&["--force"]]),
        (&["lock", "--help"], &[&["Usage:", "USAGE:"]]),
        (&["tap", "--help"], &[&["[TAP]", "[<tap>]"], &["--remove"]]),
        (&["space", "--help"], &[&["show"], &["clean"]]),
        (&["space", "show", "--help"], &[&["--details"]]),
        (&["space", "clean", "--help"], &[&["--all"], &["--dry-run"]]),
        (
            &["link", "--help"],
            &[&["<PACKAGE>", "<package>"], &["--force"]],
        ),
        (&["unlink", "--help"], &[&["<PACKAGE>", "<package>"]]),
        (&["doctor", "--help"], &[&["Usage:", "USAGE:"]]),
        (&["completions", "--help"], &[&["<SHELL>", "<shell>"]]),
    ];

    for (args, expected) in cases {
        assert_help(args, expected);
    }
}

#[test]
fn hidden_exec_command_is_still_callable_for_shims() {
    assert_help(
        &["exec", "--help"],
        &[
            &["<PACKAGE>", "<package>"],
            &["<BINARY>", "<binary>"],
            &["[ARGS]...", "[<args> ...]"],
        ],
    );
}
