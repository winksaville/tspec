use anyhow::{Context, Result, bail};
use std::fs;
use std::process::Command;

use crate::cargo_build::{apply_spec_to_command, generate_build_rs};
use crate::find_paths::{find_package_dir, find_project_root, find_tspec};
use crate::tspec::{expand_target_dir, load_spec, spec_name_from_path};
/// Check if spec requires nightly toolchain
fn requires_nightly(spec: &crate::types::Spec) -> bool {
    // High-level panic mode may require nightly
    let panic_needs_nightly = spec.panic.map(|p| p.requires_nightly()).unwrap_or(false);

    let has_build_std = !spec.rustc.build_std.is_empty();

    let has_unstable = !spec.cargo.unstable.is_empty();

    panic_needs_nightly || has_build_std || has_unstable
}

/// Test a package with a spec
pub fn test_package(pkg_name: &str, tspec: Option<&str>, release: bool) -> Result<()> {
    let workspace = find_project_root()?;
    let pkg_dir = find_package_dir(&workspace, pkg_name)?;
    let tspec_path = find_tspec(&pkg_dir, tspec)?;

    // Track if we generated a build.rs
    let build_rs_path = pkg_dir.join("build.rs");
    let had_build_rs = build_rs_path.exists();

    // Apply spec if present, otherwise plain cargo test
    let status = if let Some(path) = &tspec_path {
        let spec = load_spec(path)?;
        let spec_name = spec_name_from_path(path);
        let expanded_td = expand_target_dir(&spec, &spec_name)?;
        println!("Testing {} with spec {}", pkg_name, path.display());

        // Generate temporary build.rs for linker flags if needed
        let has_linker_args = !spec.linker.args.is_empty();
        if has_linker_args && !had_build_rs {
            generate_build_rs(&build_rs_path, pkg_name, &spec)?;
        }

        let mut cmd = Command::new("cargo");
        if requires_nightly(&spec) {
            cmd.arg("+nightly");
        }
        cmd.arg("test");
        cmd.arg("-p").arg(pkg_name);
        cmd.current_dir(&workspace);

        apply_spec_to_command(&mut cmd, &spec, &workspace, release, expanded_td.as_deref())?;
        cmd.status().context("failed to run cargo test")?
    } else {
        println!("Testing {} (no tspec)", pkg_name);
        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        cmd.arg("-p").arg(pkg_name);
        cmd.current_dir(&workspace);
        if release {
            cmd.arg("--release");
        }
        cmd.status().context("failed to run cargo test")?
    };

    // Clean up generated build.rs (only if we created it)
    if !had_build_rs && build_rs_path.exists() {
        let _ = fs::remove_file(&build_rs_path);
    }

    if !status.success() {
        bail!("cargo test failed");
    }

    Ok(())
}
