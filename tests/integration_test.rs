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
// POP fixture tests (run with: tspec test --test integration_test -- --ignored)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn pop_spec_loads_correctly() {
    let (_tmp, project) = fixture::copy_fixture("pop");
    let spec_path = project.join("tspec.ts.toml");
    let spec = load_spec(&spec_path).expect("failed to load spec");

    assert_eq!(spec.panic, Some(PanicMode::Abort));
    assert_eq!(spec.cargo.profile, Some("release".to_string()));
}

#[test]
#[ignore]
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
#[ignore]
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
#[ignore]
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
