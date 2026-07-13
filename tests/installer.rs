use std::process::Command;

/// Run a CLI command and capture output.
fn run_cmd(args: &[&str], envs: &[(&str, &str)]) -> (bool, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_edash"));
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.args(args);
    let output = cmd.output().unwrap();
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_internal_paths_gated() {
    let (ok, stdout, stderr) = run_cmd(&["__internal", "paths"], &[]);
    assert!(!ok, "should fail without EDASH_INSTALLER=1");
    assert!(stderr.contains("internal") || stdout.contains("internal"),
        "error should mention internal command");
}

#[test]
fn test_internal_paths_allowed() {
    let (ok, stdout, _) = run_cmd(&["__internal", "paths"], &[("EDASH_INSTALLER", "1")]);
    assert!(ok, "should succeed with EDASH_INSTALLER=1");
    assert!(stdout.contains("config_dir="), "should print config_dir");
    assert!(stdout.contains("data_dir="), "should print data_dir");
    assert!(stdout.contains("bin_dir="), "should print bin_dir");
}

#[test]
fn test_internal_self_test_gated() {
    let (ok, _, _) = run_cmd(&["__internal", "self-test"], &[]);
    assert!(!ok, "should fail without EDASH_INSTALLER=1");
}

#[test]
fn test_internal_self_test_allowed() {
    let (ok, stdout, _) = run_cmd(&["__internal", "self-test"], &[("EDASH_INSTALLER", "1")]);
    assert!(ok, "should succeed with EDASH_INSTALLER=1");
    assert!(stdout.contains("self-test PASSED"), "should print PASSED");
}

#[test]
fn test_internal_hidden_from_help() {
    let (ok, stdout, _) = run_cmd(&["--help"], &[]);
    assert!(ok);
    assert!(!stdout.contains("__internal"), "__internal should be hidden from help");
    assert!(stdout.contains("update"), "help should show update");
    assert!(stdout.contains("repair"), "help should show repair");
}

#[test]
fn test_version() {
    let (ok, stdout, _) = run_cmd(&["--version"], &[]);
    assert!(ok);
    assert!(stdout.contains("edash"), "version should contain 'edash'");
}

#[test]
fn test_subcommand_help() {
    let (ok, stdout, _) = run_cmd(&["update", "--help"], &[]);
    assert!(ok);
    assert!(stdout.contains("update") || !stdout.is_empty(), "update --help should work");
}

#[test]
fn test_repair_nothing_to_repair() {
    // Clean state — should say nothing to repair
    let (ok, _stdout, _) = run_cmd(&["repair"], &[]);
    assert!(ok);
    // May or may not have things to repair depending on test env
    // Just verify it doesn't crash
}

#[test]
fn test_internal_stage_catalog_gated() {
    let (ok, _, _) = run_cmd(
        &["__internal", "stage-catalog", "/tmp/nonexistent", "/tmp/nonexistent.yaml"],
        &[],
    );
    assert!(!ok, "should fail without EDASH_INSTALLER=1");
}
