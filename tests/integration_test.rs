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

// ---------------------------------------------------------------------------
// POP+WS fixture tests
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn pop_ws_spec_loads_correctly() {
    let (_tmp, project) = fixture::copy_fixture("pop-ws");
    let spec_path = project.join("tspec.ts.toml");
    let spec = load_spec(&spec_path).expect("failed to load spec");

    assert_eq!(spec.panic, Some(PanicMode::Abort));
    assert_eq!(spec.cargo.profile, Some("release".to_string()));
}

#[test]
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
