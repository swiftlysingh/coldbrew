use clap::Parser;
use coldbrew::cli::{Cli, Commands};

#[test]
fn install_lock_accepts_force_without_packages() {
    let cli = Cli::try_parse_from(["crew", "install", "--lock", "--force"]).unwrap();

    match cli.command {
        Some(Commands::Install {
            packages,
            lock,
            skip_deps,
            force,
        }) => {
            assert!(packages.is_empty());
            assert!(lock);
            assert!(!skip_deps);
            assert!(force);
        }
        _ => panic!("expected install command"),
    }
}

#[test]
fn install_lock_conflicts_with_explicit_packages() {
    let err = match Cli::try_parse_from(["crew", "install", "--lock", "jq"]) {
        Ok(_) => panic!("expected parse error for conflicting --lock and package args"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
}

#[test]
fn install_requires_packages_when_lock_not_set() {
    let err = match Cli::try_parse_from(["crew", "install"]) {
        Ok(_) => panic!("expected parse error when install has no packages and no --lock"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
}
