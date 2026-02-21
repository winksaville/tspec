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
