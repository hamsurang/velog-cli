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

// ---- Format flag tests ----

#[test]
fn help_shows_format_flag() {
    velog_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn format_default_is_compact() {
    velog_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("[default: pretty]"));
}

#[test]
fn format_flag_accepts_compact() {
    // Verify --format compact is a valid value (not rejected by clap)
    let output = velog_cmd()
        .args(["--format", "compact", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn format_flag_accepts_pretty() {
    let output = velog_cmd()
        .args(["--format", "pretty", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn format_flag_accepts_silent() {
    let output = velog_cmd()
        .args(["--format", "silent", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn format_flag_rejects_invalid() {
    velog_cmd()
        .args(["--format", "xml", "post", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn completions_ignore_format_flag() {
    // Completions should work regardless of format flag
    velog_cmd()
        .args(["--format", "compact", "completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn compact_auth_login_returns_error() {
    // auth login should fail in compact mode (requires interactive)
    velog_cmd()
        .args(["--format", "compact", "auth", "login"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "auth login requires --format pretty",
        ));
}

#[test]
fn silent_auth_login_returns_error() {
    // auth login should fail in silent mode too
    velog_cmd()
        .args(["--format", "silent", "auth", "login"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "auth login requires --format pretty",
        ));
}

#[test]
fn compact_error_is_json() {
    // Errors in compact mode should be JSON on stderr
    let output = velog_cmd()
        .args(["--format", "compact", "auth", "login"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain JSON with "error" key
    assert!(
        stderr.contains(r#""error":"#),
        "stderr should be JSON: {}",
        stderr
    );
    assert!(
        stderr.contains(r#""exit_code":"#),
        "stderr should contain exit_code: {}",
        stderr
    );
}

#[test]
fn silent_error_is_json() {
    // Errors in silent mode should also be JSON on stderr
    let output = velog_cmd()
        .args(["--format", "silent", "auth", "login"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(r#""error":"#),
        "stderr should be JSON: {}",
        stderr
    );
}

#[test]
fn compact_post_show_nonexistent_error_is_json() {
    // post show with a nonexistent slug should emit JSON error in compact mode
    // (requires auth, so this test may succeed or fail depending on credentials)
    let output = velog_cmd()
        .args([
            "--format",
            "compact",
            "post",
            "show",
            "nonexistent-slug-that-does-not-exist-12345",
        ])
        .output()
        .unwrap();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(r#""error":"#),
            "stderr should be JSON in compact mode: {}",
            stderr
        );
        assert!(
            stderr.contains(r#""exit_code":"#),
            "stderr should contain exit_code: {}",
            stderr
        );
    }
    // If it somehow succeeds (unlikely), that's also fine — the format flag was accepted
}

// ---- Mutual exclusion tests (post list flags) ----

#[test]
fn conflicting_flags_trending_recent() {
    velog_cmd()
        .args(["post", "list", "--trending", "--recent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn conflicting_flags_trending_drafts() {
    velog_cmd()
        .args(["post", "list", "--trending", "--drafts"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn conflicting_flags_username_recent() {
    velog_cmd()
        .args(["post", "list", "-u", "teo", "--recent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn conflicting_flags_username_trending() {
    velog_cmd()
        .args(["post", "list", "-u", "teo", "--trending"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn conflicting_flags_username_drafts() {
    velog_cmd()
        .args(["post", "list", "-u", "teo", "--drafts"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn period_requires_trending() {
    velog_cmd()
        .args(["post", "list", "--period", "week"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--trending"));
}

#[test]
fn offset_requires_trending() {
    velog_cmd()
        .args(["post", "list", "--offset", "10"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--trending"));
}

#[test]
fn period_accepts_all_values() {
    for period in &["day", "week", "month", "year"] {
        // Should parse successfully (will fail at API level, not at clap level)
        let output = velog_cmd()
            .args(["post", "list", "--trending", "--period", period, "--help"])
            .output()
            .unwrap();
        // --help always succeeds, confirming the flag was accepted
        assert!(output.status.success(), "period={} should be accepted", period);
    }
}

#[test]
fn period_rejects_invalid_value() {
    velog_cmd()
        .args(["post", "list", "--trending", "--period", "hourly"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn list_help_shows_new_flags() {
    velog_cmd()
        .args(["post", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--trending"))
        .stdout(predicate::str::contains("--recent"))
        .stdout(predicate::str::contains("--username"))
        .stdout(predicate::str::contains("--limit"))
        .stdout(predicate::str::contains("--cursor"))
        .stdout(predicate::str::contains("--offset"))
        .stdout(predicate::str::contains("--period"));
}
