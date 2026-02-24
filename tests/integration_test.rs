mod fixture;

use std::process::Command;

use tspec::options::PanicMode;
use tspec::tspec::load_spec;

/// Find the tspec binary (built by cargo).
fn tspec_bin() -> String {
    // Use the installed tspec binary
    "tspec".to_string()
}

// ---------------------------------------------------------------------------
// POP fixture tests
// ---------------------------------------------------------------------------

#[test]
fn pop_spec_loads_correctly() {
    let (_tmp, project) = fixture::copy_fixture("pop");
    let spec_path = project.join("tspec.ts.toml");
    let spec = load_spec(&spec_path).expect("failed to load spec");

    assert_eq!(spec.panic, Some(PanicMode::Abort));
    assert_eq!(spec.cargo.profile, Some("release".to_string()));
}

#[test]
fn pop_cargo_build_succeeds() {
    let (_tmp, project) = fixture::copy_fixture("pop");

    let output = Command::new("cargo")
        .args(["build"])
        .current_dir(&project)
        .output()
        .expect("failed to run cargo build");

    assert!(
        output.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pop_tspec_build_succeeds() {
    let (_tmp, project) = fixture::copy_fixture("pop");

    let output = Command::new(tspec_bin())
        .args(["build", "."])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build");

    assert!(
        output.status.success(),
        "tspec build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pop_tspec_compare_succeeds() {
    let (_tmp, project) = fixture::copy_fixture("pop");

    // Build first so compare has something to compare
    let build = Command::new(tspec_bin())
        .args(["build", "."])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build");
    assert!(
        build.status.success(),
        "tspec build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let output = Command::new(tspec_bin())
        .args(["compare", "."])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec compare");

    assert!(
        output.status.success(),
        "tspec compare failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// POP+WS fixture tests
// ---------------------------------------------------------------------------

#[test]
fn pop_ws_spec_loads_correctly() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");
    let spec_path = project.join("tspec.ts.toml");
    let spec = load_spec(&spec_path).expect("failed to load spec");

    assert_eq!(spec.panic, Some(PanicMode::Abort));
    assert_eq!(spec.cargo.profile, Some("release".to_string()));
}

#[test]
fn pop_ws_cargo_build_succeeds() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");

    let output = Command::new("cargo")
        .args(["build"])
        .current_dir(&project)
        .output()
        .expect("failed to run cargo build");

    assert!(
        output.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pop_ws_tspec_build_root_package() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");

    let output = Command::new(tspec_bin())
        .args(["build", "."])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build");

    assert!(
        output.status.success(),
        "tspec build . failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pop_ws_tspec_build_member_by_name() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");

    let output = Command::new(tspec_bin())
        .args(["build", "-p", "mylib"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build -p mylib");

    assert!(
        output.status.success(),
        "tspec build -p mylib failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pop_ws_tspec_build_all() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");

    let output = Command::new(tspec_bin())
        .args(["build", "-w"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build -w");

    assert!(
        output.status.success(),
        "tspec build -w failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pop_ws_tspec_build_from_member_dir() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");

    let output = Command::new(tspec_bin())
        .args(["build", "."])
        .current_dir(project.join("mylib"))
        .output()
        .expect("failed to run tspec build from mylib dir");

    assert!(
        output.status.success(),
        "tspec build . from mylib dir failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// POWS fixture tests (Pure Old Workspace — no root [package])
// ---------------------------------------------------------------------------

#[test]
fn pows_cargo_build_succeeds() {
    let (_tmp, project) = fixture::copy_fixture("pows");

    let output = Command::new("cargo")
        .args(["build"])
        .current_dir(&project)
        .output()
        .expect("failed to run cargo build");

    assert!(
        output.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pows_tspec_build_all_from_root() {
    let (_tmp, project) = fixture::copy_fixture("pows");

    // At a POWS root with no args, tspec should build all members
    let output = Command::new(tspec_bin())
        .args(["build"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build");

    assert!(
        output.status.success(),
        "tspec build (all) failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pows_tspec_build_member_by_name() {
    let (_tmp, project) = fixture::copy_fixture("pows");

    let output = Command::new(tspec_bin())
        .args(["build", "-p", "pows-app"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build -p pows-app");

    assert!(
        output.status.success(),
        "tspec build -p pows-app failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pows_tspec_test_all_from_root() {
    let (_tmp, project) = fixture::copy_fixture("pows");

    let output = Command::new(tspec_bin())
        .args(["test"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec test");

    assert!(
        output.status.success(),
        "tspec test (all) failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pows_tspec_build_from_member_dir() {
    let (_tmp, project) = fixture::copy_fixture("pows");

    let output = Command::new(tspec_bin())
        .args(["build", "."])
        .current_dir(project.join("pows-app"))
        .output()
        .expect("failed to run tspec build from app dir");

    assert!(
        output.status.success(),
        "tspec build . from app dir failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pows_tspec_dot_resolves_to_all_at_root() {
    let (_tmp, project) = fixture::copy_fixture("pows");

    // "tspec build ." at a POWS root should resolve to all-packages
    // (because . has no [package])
    let output = Command::new(tspec_bin())
        .args(["build", "."])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build .");

    assert!(
        output.status.success(),
        "tspec build . at POWS root failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Fail fixture tests (run with `tspec test -- --ignored`)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn pop_fail_test_exits_nonzero() {
    let (_tmp, project) = fixture::copy_fixture("pop-fail");

    let output = Command::new(tspec_bin())
        .args(["test", "."])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec test");

    assert!(
        !output.status.success(),
        "tspec test should fail but succeeded"
    );
}

#[test]
#[ignore]
fn pop_fail_test_shows_failure_counts() {
    let (_tmp, project) = fixture::copy_fixture("pop-fail");

    let output = Command::new(tspec_bin())
        .args(["test", "."])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec test");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // cargo test output should contain a FAILED result line with 1 failed
    assert!(
        stdout.contains("1 failed"),
        "expected '1 failed' in stdout:\n{}",
        stdout
    );
}

#[test]
#[ignore]
fn pows_fail_test_exits_nonzero() {
    let (_tmp, project) = fixture::copy_fixture("pows-fail");

    let output = Command::new(tspec_bin())
        .args(["test"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec test");

    assert!(
        !output.status.success(),
        "tspec test should fail but succeeded"
    );
}

#[test]
#[ignore]
fn pows_fail_test_summary_shows_mixed_results() {
    let (_tmp, project) = fixture::copy_fixture("pows-fail");

    let output = Command::new(tspec_bin())
        .args(["test"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec test");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Summary should show per-package counts with mixed pass/fail
    assert!(
        stdout.contains("[PASS]"),
        "expected [PASS] in summary:\n{}",
        stdout
    );
    assert!(
        stdout.contains("[FAIL]"),
        "expected [FAIL] in summary:\n{}",
        stdout
    );
    // Footer should show aggregate counts
    assert!(
        stdout.contains("passed") && stdout.contains("failed"),
        "expected aggregate counts in footer:\n{}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// POPWS-3P fixture tests (workspace with 3 packages, mixed targets)
// ---------------------------------------------------------------------------

#[test]
fn popws3p_build_all() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");

    let output = Command::new(tspec_bin())
        .args(["build"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec build (all) failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("app-a"), "missing app-a in output");
    assert!(stdout.contains("lib-b"), "missing lib-b in output");
    assert!(stdout.contains("multi-c"), "missing multi-c in output");
}

#[test]
fn popws3p_build_single_package() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");

    let output = Command::new(tspec_bin())
        .args(["build", "-p", "app-a"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build -p app-a");

    assert!(
        output.status.success(),
        "tspec build -p app-a failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn popws3p_test_all() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");

    let output = Command::new(tspec_bin())
        .args(["test"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec test");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec test (all) failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("app-a"), "missing app-a in output");
    assert!(stdout.contains("lib-b"), "missing lib-b in output");
    assert!(stdout.contains("multi-c"), "missing multi-c in output");
    assert!(stdout.contains("7 passed"), "expected 7 passed:\n{stdout}");
    assert!(stdout.contains("0 failed"), "expected 0 failed:\n{stdout}");
}

// ---------------------------------------------------------------------------
// --manifest-path / --mp tests
// ---------------------------------------------------------------------------

#[test]
fn mp_build_workspace_dir() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");

    let output = Command::new(tspec_bin())
        .args(["build", "--mp", project.to_str().unwrap()])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("failed to run tspec build --mp");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec build --mp failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("app-a"), "missing app-a");
    assert!(stdout.contains("multi-c"), "missing multi-c");
}

#[test]
fn mp_build_single_package() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");

    let output = Command::new(tspec_bin())
        .args(["build", "--mp", project.to_str().unwrap(), "-p", "app-a"])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("failed to run tspec build --mp -p app-a");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec build --mp -p app-a failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("app-a"), "missing app-a");
}

#[test]
fn mp_build_subpackage_dir() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");
    let app_a_dir = project.join("app-a");

    let output = Command::new(tspec_bin())
        .args(["build", "--mp", app_a_dir.to_str().unwrap()])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("failed to run tspec build --mp app-a/");

    // Should resolve to workspace root and build all
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec build --mp app-a/ failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("app-a"), "missing app-a");
    assert!(stdout.contains("multi-c"), "missing multi-c");
}

#[test]
fn mp_build_cargo_toml_path() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");
    let cargo_toml = project.join("lib-b").join("Cargo.toml");

    let output = Command::new(tspec_bin())
        .args(["build", "--mp", cargo_toml.to_str().unwrap()])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("failed to run tspec build --mp Cargo.toml");

    // Should resolve to workspace root and build all
    assert!(
        output.status.success(),
        "tspec build --mp Cargo.toml failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mp_test_workspace() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");

    let output = Command::new(tspec_bin())
        .args(["test", "--mp", project.to_str().unwrap()])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("failed to run tspec test --mp");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec test --mp failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("TEST SUMMARY"), "missing summary");
    assert!(stdout.contains("7 passed"), "expected 7 passed:\n{stdout}");
}

#[test]
fn mp_pop_fixture() {
    let (_tmp, project) = fixture::copy_fixture("pop");

    let output = Command::new(tspec_bin())
        .args(["build", "--mp", project.to_str().unwrap()])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("failed to run tspec build --mp pop");

    assert!(
        output.status.success(),
        "tspec build --mp pop failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Version in SUMMARY tests
// ---------------------------------------------------------------------------

#[test]
fn pop_ws_build_summary_shows_versions() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");

    let output = Command::new(tspec_bin())
        .args(["build", "-w"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec build -w");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec build -w failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Header should include root package version
    assert!(
        stdout.contains("pop-ws v0.3.0 BUILD SUMMARY"),
        "missing versioned header in:\n{stdout}"
    );
    // Rows should show per-package versions
    assert!(
        stdout.contains("pop-ws-app v0.3.0"),
        "missing pop-ws-app version in:\n{stdout}"
    );
    assert!(
        stdout.contains("mylib v0.2.0"),
        "missing mylib version in:\n{stdout}"
    );
}

#[test]
fn popws3p_test_summary_shows_versions() {
    let (_tmp, project) = fixture::copy_fixture("popws-3p");

    let output = Command::new(tspec_bin())
        .args(["test"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec test");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec test failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Virtual workspace — header should NOT have a version
    assert!(
        stdout.contains("popws-3p TEST SUMMARY"),
        "missing header in:\n{stdout}"
    );
    // Rows should show per-package versions
    assert!(
        stdout.contains("app-a v0.4.0"),
        "missing app-a version in:\n{stdout}"
    );
    assert!(
        stdout.contains("lib-b v0.5.0"),
        "missing lib-b version in:\n{stdout}"
    );
    assert!(
        stdout.contains("multi-c v0.6.0"),
        "missing multi-c version in:\n{stdout}"
    );
}

#[test]
fn pop_ws_run_summary_shows_versions() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");

    let output = Command::new(tspec_bin())
        .args(["run", "-w"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec run -w");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec run -w failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("pop-ws v0.3.0 RUN SUMMARY"),
        "missing versioned header in:\n{stdout}"
    );
    assert!(
        stdout.contains("pop-ws-app v0.3.0"),
        "missing pop-ws-app version in:\n{stdout}"
    );
}

#[test]
fn pop_ws_compare_summary_shows_versions() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");

    let output = Command::new(tspec_bin())
        .args(["compare", "-w"])
        .current_dir(&project)
        .output()
        .expect("failed to run tspec compare -w");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tspec compare -w failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Single binary package — should show per-package compare header with version
    assert!(
        stdout.contains("pop-ws-app v0.3.0 COMPARE SUMMARY"),
        "missing versioned compare header in:\n{stdout}"
    );
}
