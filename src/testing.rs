use anyhow::{Context, Result, bail};
use std::fs;
use std::process::Command;

use crate::cargo_build::{
    apply_spec_to_command, check_spec_misconfigurations, generate_build_rs,
    remove_stale_tspec_build_rs, reprint_warnings, validate_profile, warn_stale_build_rs,
};
use crate::find_paths::{find_package_dir, find_project_root, find_tspec, get_package_name};
use crate::tspec::{expand_target_dir, load_spec, spec_name_from_path};
/// Check if spec requires nightly toolchain for testing.
/// This is stricter than the build version: panic=abort also needs nightly
/// because `-Zpanic_abort_tests` is a nightly-only flag.
fn requires_nightly_for_test(spec: &crate::types::Spec) -> bool {
    let panic_needs_nightly = spec
        .panic
        .map(|p| p.rustc_panic_value().is_some()) // any non-unwind panic needs nightly for tests
        .unwrap_or(false);

    let has_build_std = !spec.cargo.build_std.is_empty();

    let has_unstable = !spec.cargo.unstable.is_empty();

    panic_needs_nightly || has_build_std || has_unstable
}

/// Check if spec uses an abort-like panic mode that needs `-Zpanic_abort_tests` for testing.
fn needs_panic_abort_tests(spec: &crate::types::Spec) -> bool {
    spec.panic
        .map(|p| p.rustc_panic_value().is_some()) // abort or immediate-abort
        .unwrap_or(false)
}

/// Test a package with a spec.
/// `cli_profile` is the CLI-specified profile (None = debug default).
pub fn test_package(pkg_name: &str, tspec: Option<&str>, cli_profile: Option<&str>) -> Result<()> {
    let workspace = find_project_root()?;
    let pkg_dir = find_package_dir(&workspace, pkg_name)?;
    let tspec_path = find_tspec(&pkg_dir, tspec)?;

    // Resolve actual package name from Cargo.toml (needed when pkg_name is a path)
    let pkg_name = get_package_name(&pkg_dir)?;

    // Clean up any stale tspec-generated build.rs from interrupted builds
    let had_stale_build_rs = remove_stale_tspec_build_rs(&pkg_dir);

    // Track if we generated a build.rs
    let build_rs_path = pkg_dir.join("build.rs");
    let had_build_rs = build_rs_path.exists();

    // Apply spec if present, otherwise plain cargo test
    let (status, spec_warnings) = if let Some(path) = &tspec_path {
        let spec = load_spec(path)?;
        let spec_name = spec_name_from_path(path);
        let expanded_td = expand_target_dir(&spec, &spec_name)?;

        // Validate the effective profile before invoking cargo
        let effective = spec.cargo.profile.as_deref().or(cli_profile);
        if let Some(profile) = effective {
            validate_profile(profile, &workspace)?;
        }
        println!("Testing {} with spec {}", pkg_name, path.display());

        let spec_warnings = check_spec_misconfigurations(&pkg_name, &spec, &pkg_dir);

        // Generate temporary build.rs for linker flags if needed
        let has_linker_args = !spec.linker.args.is_empty();
        let has_bin_target = pkg_dir.join("src/main.rs").exists();
        if has_linker_args && has_bin_target && !had_build_rs {
            generate_build_rs(&build_rs_path, &pkg_name, &spec)?;
        }

        let mut cmd = Command::new("cargo");
        if requires_nightly_for_test(&spec) {
            cmd.arg("+nightly");
        }
        cmd.arg("test");
        cmd.arg("-p").arg(&pkg_name);
        cmd.current_dir(&workspace);

        // Set spec path for tspec-build library to read in build.rs
        cmd.env("TSPEC_SPEC_FILE", path.as_os_str());

        apply_spec_to_command(
            &mut cmd,
            &spec,
            &workspace,
            cli_profile,
            expanded_td.as_deref(),
        )?;

        // Append -Zpanic_abort_tests to RUSTFLAGS if needed (must come after
        // apply_spec_to_command which may set RUSTFLAGS)
        if needs_panic_abort_tests(&spec) {
            let existing = cmd
                .get_envs()
                .find(|(k, _)| k == &"RUSTFLAGS")
                .and_then(|(_, v)| v)
                .map(|v| v.to_string_lossy().into_owned())
                .unwrap_or_default();
            let new_flags = if existing.is_empty() {
                "-Zpanic_abort_tests".to_string()
            } else {
                format!("{} -Zpanic_abort_tests", existing)
            };
            cmd.env("RUSTFLAGS", new_flags);
        }

        (
            cmd.status().context("failed to run cargo test")?,
            spec_warnings,
        )
    } else {
        // Validate CLI profile when no spec
        if let Some(profile) = cli_profile {
            validate_profile(profile, &workspace)?;
        }
        println!("Testing {} (no tspec)", pkg_name);
        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        cmd.arg("-p").arg(&pkg_name);
        cmd.current_dir(&workspace);
        if let Some(p) = cli_profile {
            match p {
                "debug" | "dev" => {}
                _ => {
                    cmd.arg("--profile").arg(p);
                }
            }
        }
        (
            cmd.status().context("failed to run cargo test")?,
            Vec::new(),
        )
    };

    // Clean up generated build.rs (only if we created it)
    if !had_build_rs && build_rs_path.exists() {
        let _ = fs::remove_file(&build_rs_path);
    }

    if !status.success() {
        match &tspec_path {
            Some(path) => {
                let display_path = path.strip_prefix(&workspace).unwrap_or(path).display();
                bail!(
                    "cargo test failed for `{}` with spec {}",
                    pkg_name,
                    display_path
                )
            }
            None => bail!("cargo test failed for `{}`", pkg_name),
        }
    }

    warn_stale_build_rs(had_stale_build_rs);
    reprint_warnings(&spec_warnings);
    Ok(())
}
