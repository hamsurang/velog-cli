use assert_cmd::Command;
use predicates::prelude::*;

fn velog_cmd() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("velog").unwrap()
}

#[test]
fn help_shows_usage() {
    velog_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("CLI client for velog.io"));
}

#[test]
fn version_shows_package_version() {
    velog_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("velog"));
}

#[test]
fn auth_subcommand_help() {
    velog_cmd()
        .args(["auth", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Authentication"));
}

#[test]
fn post_subcommand_help() {
    velog_cmd()
        .args(["post", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Post management"));
}

#[test]
fn unknown_subcommand_fails() {
    velog_cmd().arg("nonexistent").assert().failure();
}

#[test]
fn completions_bash() {
    velog_cmd()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn completions_zsh() {
    velog_cmd()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn post_create_requires_title() {
    velog_cmd()
        .args(["post", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--title"));
}
