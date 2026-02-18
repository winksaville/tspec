//! Batch operations on all workspace packages
//!
//! Provides build_all, run_all, test_all for operating on all workspace members.

use std::os::unix::fs::PermissionsExt;
use std::process::ExitCode;

use std::path::{Path, PathBuf};

use crate::binary::{binary_size, strip_binary};
use crate::cargo_build::build_package;
use crate::compare::{SpecResult, compare_specs, print_comparison};
use crate::find_paths::find_tspecs;
use crate::run::run_binary;
use crate::testing::test_package;
use crate::workspace::{PackageKind, WorkspaceInfo};
use crate::{print_header, print_hline};

/// Resolve tspec patterns for a workspace member.
///
/// Returns the matching spec paths, or an empty vec if no patterns match.
/// When `patterns` is empty, returns an empty vec (caller should use default behavior).
fn resolve_specs_for_member(member_path: &Path, patterns: &[String]) -> Vec<PathBuf> {
    if patterns.is_empty() {
        return Vec::new();
    }
    find_tspecs(member_path, patterns).unwrap_or_default()
}

/// Result of a batch operation on a single package
pub struct OpResult {
    pub name: String,
    pub success: bool,
    pub message: String,
    pub size: Option<u64>,
}

/// Build all workspace packages (excluding build tools)
///
/// When `tspec_patterns` is empty, each package uses its default spec.
/// When non-empty, patterns are resolved per-package; packages with no matches are skipped.
pub fn build_all(
    workspace: &WorkspaceInfo,
    tspec_patterns: &[String],
    cli_profile: Option<&str>,
    strip: bool,
    fail_fast: bool,
) -> Vec<OpResult> {
    let mut results = Vec::new();

    for member in workspace.buildable_members() {
        let specs = resolve_specs_for_member(&member.path, tspec_patterns);
        if specs.is_empty() && !tspec_patterns.is_empty() {
            continue;
        }

        println!("=== {} ===", member.name);

        let tspec_list: Vec<Option<String>> = if specs.is_empty() {
            vec![None]
        } else {
            specs
                .into_iter()
                .map(|p| Some(p.to_string_lossy().into_owned()))
                .collect()
        };

        for tspec in &tspec_list {
            let result = match build_package(&member.name, tspec.as_deref(), cli_profile) {
                Ok(build_result) => {
                    if strip
                        && member.has_binary
                        && let Err(e) = strip_binary(&build_result.binary_path)
                    {
                        eprintln!("  warning: strip failed: {}", e);
                    }
                    let size = binary_size(&build_result.binary_path).ok();
                    OpResult {
                        name: member.name.clone(),
                        success: true,
                        message: format!("{}", build_result.binary_path.display()),
                        size,
                    }
                }
                Err(e) => OpResult {
                    name: member.name.clone(),
                    success: false,
                    message: e.to_string(),
                    size: None,
                },
            };

            let failed = !result.success;
            results.push(result);

            if failed && fail_fast {
                return results;
            }
        }
    }

    results
}

/// Run all app packages sequentially
///
/// When `tspec_patterns` is empty, each package uses its default spec.
/// When non-empty, patterns are resolved per-package; packages with no matches are skipped.
pub fn run_all(
    workspace: &WorkspaceInfo,
    tspec_patterns: &[String],
    cli_profile: Option<&str>,
    strip: bool,
) -> Vec<OpResult> {
    let mut results = Vec::new();

    for member in workspace.runnable_members() {
        let specs = resolve_specs_for_member(&member.path, tspec_patterns);
        if specs.is_empty() && !tspec_patterns.is_empty() {
            continue;
        }

        println!("=== {} ===", member.name);

        let tspec_list: Vec<Option<String>> = if specs.is_empty() {
            vec![None]
        } else {
            specs
                .into_iter()
                .map(|p| Some(p.to_string_lossy().into_owned()))
                .collect()
        };

        for tspec in &tspec_list {
            let result = match build_package(&member.name, tspec.as_deref(), cli_profile) {
                Ok(build_result) => {
                    if strip && let Err(e) = strip_binary(&build_result.binary_path) {
                        eprintln!("  warning: strip failed: {}", e);
                    }
                    match run_binary(&build_result.binary_path, &[]) {
                        Ok(exit_code) => OpResult {
                            name: member.name.clone(),
                            success: true,
                            message: format!("exit code: {}", exit_code),
                            size: None,
                        },
                        Err(e) => OpResult {
                            name: member.name.clone(),
                            success: false,
                            message: format!("run failed: {}", e),
                            size: None,
                        },
                    }
                }
                Err(e) => OpResult {
                    name: member.name.clone(),
                    success: false,
                    message: format!("build failed: {}", e),
                    size: None,
                },
            };

            results.push(result);
        }
    }

    results
}

/// Test all workspace packages
///
/// When `tspec_patterns` is empty, each package uses its default spec.
/// When non-empty, patterns are resolved per-package; packages with no matches are skipped.
pub fn test_all(
    workspace: &WorkspaceInfo,
    tspec_patterns: &[String],
    cli_profile: Option<&str>,
    fail_fast: bool,
) -> Vec<OpResult> {
    let mut results = Vec::new();

    // Test regular packages (excluding Test kind which needs special handling)
    for member in workspace.buildable_members() {
        if member.kind == PackageKind::Test {
            continue; // Handle test packages separately
        }

        let specs = resolve_specs_for_member(&member.path, tspec_patterns);
        if specs.is_empty() && !tspec_patterns.is_empty() {
            continue;
        }

        println!("=== {} ===", member.name);

        let tspec_list: Vec<Option<String>> = if specs.is_empty() {
            vec![None]
        } else {
            specs
                .into_iter()
                .map(|p| Some(p.to_string_lossy().into_owned()))
                .collect()
        };

        for tspec in &tspec_list {
            let result = match test_package(&member.name, tspec.as_deref(), cli_profile) {
                Ok(()) => OpResult {
                    name: member.name.clone(),
                    success: true,
                    message: "ok".to_string(),
                    size: None,
                },
                Err(e) => OpResult {
                    name: member.name.clone(),
                    success: false,
                    message: e.to_string(),
                    size: None,
                },
            };

            let failed = !result.success;
            results.push(result);

            if failed && fail_fast {
                return results;
            }
        }
    }

    // Handle test packages (like rlibc-x2-tests) - build and run all test binaries
    for member in workspace.test_members() {
        let specs = resolve_specs_for_member(&member.path, tspec_patterns);
        if specs.is_empty() && !tspec_patterns.is_empty() {
            continue;
        }

        println!("=== {} ===", member.name);

        // Build the test package (builds all binaries)
        let build_tspec = specs.first().map(|p| p.to_string_lossy().into_owned());
        let build_result = match build_package(&member.name, build_tspec.as_deref(), cli_profile) {
            Ok(r) => r,
            Err(e) => {
                results.push(OpResult {
                    name: member.name.clone(),
                    success: false,
                    message: format!("build failed: {}", e),
                    size: None,
                });
                if fail_fast {
                    return results;
                }
                continue;
            }
        };

        // Find and run all test binaries in the target directory
        let profile_dir = crate::types::profile_dir_name(cli_profile.unwrap_or("debug"));
        let target_dir = build_result.target_base.join(profile_dir);

        // Look for binaries that end with "-tests" in the target directory
        let test_binaries: Vec<_> = std::fs::read_dir(&target_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_file = e.path().is_file();
                let is_test = name.ends_with("-tests");
                let is_executable = e
                    .metadata()
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false);
                is_file && is_test && is_executable
            })
            .collect();

        if test_binaries.is_empty() {
            results.push(OpResult {
                name: member.name.clone(),
                success: false,
                message: "no test binaries found".to_string(),
                size: None,
            });
            continue;
        }

        for entry in test_binaries {
            let path = entry.path();
            let bin_name = entry.file_name().to_string_lossy().to_string();

            print!("  Running {}... ", bin_name);

            let result = match run_binary(&path, &[]) {
                Ok(exit_code) => {
                    if exit_code == 0 {
                        println!("ok");
                        OpResult {
                            name: format!("{}/{}", member.name, bin_name),
                            success: true,
                            message: "ok".to_string(),
                            size: None,
                        }
                    } else {
                        println!("FAILED (exit {})", exit_code);
                        OpResult {
                            name: format!("{}/{}", member.name, bin_name),
                            success: false,
                            message: format!("exit code: {}", exit_code),
                            size: None,
                        }
                    }
                }
                Err(e) => {
                    println!("FAILED");
                    OpResult {
                        name: format!("{}/{}", member.name, bin_name),
                        success: false,
                        message: format!("run failed: {}", e),
                        size: None,
                    }
                }
            };

            let failed = !result.success;
            results.push(result);

            if failed && fail_fast {
                return results;
            }
        }
    }

    results
}

/// Print a summary of operation results (for tests)
pub fn print_test_summary(results: &[OpResult]) -> ExitCode {
    let max_name_len = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(5)
        .max(5);

    println!();
    print_header!(format!("tspec {} TEST SUMMARY", env!("CARGO_PKG_VERSION")));
    println!("  {:width$}  Status", "Package", width = max_name_len);

    let mut passed = 0;
    let mut failed = 0;

    for result in results {
        let status = if result.success {
            passed += 1;
            "[PASS]"
        } else {
            failed += 1;
            "[FAIL]"
        };
        println!("  {:width$}  {status}", result.name, width = max_name_len);
    }

    println!();
    println!("  Test: {} passed, {} failed", passed, failed);
    println!();
    println!("  Note: Run `tspec test -p tspec` or `cargo test -p tspec` to test tspec itself");
    print_hline!();
    println!();

    if failed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Print a summary for build operations (OK/FAILED)
pub fn print_summary(results: &[OpResult]) -> ExitCode {
    let max_name_len = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(5)
        .max(5);

    println!();
    print_header!("BUILD SUMMARY");
    println!(
        "  {:width$}  Status    Size",
        "Package",
        width = max_name_len
    );

    let mut ok_count = 0;
    let mut failed_count = 0;

    for result in results {
        let status = if result.success {
            ok_count += 1;
            "[ OK ]"
        } else {
            failed_count += 1;
            "[FAIL]"
        };
        let size_str = result
            .size
            .map(format_size)
            .unwrap_or_else(|| "--".to_string());
        println!(
            "  {:width$}  {status}  {:>6}",
            result.name,
            size_str,
            width = max_name_len
        );
    }

    println!();
    println!("  Build: {} ok, {} failed", ok_count, failed_count);
    print_hline!();
    println!();

    if failed_count > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1}M", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1}K", bytes as f64 / 1_000.0)
    } else {
        format!("{}", bytes)
    }
}

/// Result of a compare operation on a single package
pub struct CompareResult {
    pub op: OpResult,
    pub specs: Vec<SpecResult>,
}

/// Compare all workspace packages that have binaries
///
/// When `tspec_patterns` is empty, each package discovers its own specs via default glob.
/// When non-empty, patterns are resolved per-package; packages with no matches are skipped.
pub fn compare_all(
    workspace: &WorkspaceInfo,
    tspec_patterns: &[String],
    fail_fast: bool,
) -> Vec<CompareResult> {
    let mut results = Vec::new();

    for member in workspace.buildable_members() {
        if !member.has_binary {
            continue;
        }

        let spec_paths = if tspec_patterns.is_empty() {
            find_tspecs(&member.path, &[]).unwrap_or_default()
        } else {
            let resolved = resolve_specs_for_member(&member.path, tspec_patterns);
            if resolved.is_empty() {
                continue;
            }
            resolved
        };

        println!("=== {} ===", member.name);

        let (op, specs) = match compare_specs(&member.name, &spec_paths) {
            Ok(spec_results) => (
                OpResult {
                    name: member.name.clone(),
                    success: true,
                    message: "ok".to_string(),
                    size: None,
                },
                spec_results,
            ),
            Err(e) => (
                OpResult {
                    name: member.name.clone(),
                    success: false,
                    message: e.to_string(),
                    size: None,
                },
                Vec::new(),
            ),
        };

        let failed = !op.success;
        results.push(CompareResult { op, specs });

        if failed && fail_fast {
            break;
        }
    }

    results
}

/// Print a summary for compare operations
///
/// With a single package, just prints its comparison table.
/// With multiple packages, reprints all per-package tables then an overall OK/FAIL summary.
pub fn print_compare_summary(results: &[CompareResult]) -> ExitCode {
    let has_failure = results.iter().any(|r| !r.op.success);

    // Reprint per-package comparison tables together
    for result in results {
        if !result.specs.is_empty() {
            print_comparison(&result.op.name, &result.specs);
        }
    }

    // Only show overall COMPARE SUMMARY when there are multiple packages
    if results.len() > 1 {
        let max_name_len = results
            .iter()
            .map(|r| r.op.name.len())
            .max()
            .unwrap_or(5)
            .max(5);

        print_header!("COMPARE SUMMARY");
        println!("  {:width$}  Status", "Package", width = max_name_len);

        let mut ok_count = 0;
        let mut failed_count = 0;

        for result in results {
            let status = if result.op.success {
                ok_count += 1;
                "[ OK ]"
            } else {
                failed_count += 1;
                "[FAIL]"
            };
            println!(
                "  {:width$}  {status}",
                result.op.name,
                width = max_name_len
            );
        }

        println!();
        println!("  Compare: {} ok, {} failed", ok_count, failed_count);
        print_hline!();
        println!();
    }

    if has_failure {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Print a summary for run operations (shows exit codes, not pass/fail)
pub fn print_run_summary(results: &[OpResult]) -> ExitCode {
    let max_name_len = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(5)
        .max(5);

    println!();
    print_header!("RUN SUMMARY");
    println!("  {:width$}  Exit", "Package", width = max_name_len);

    let mut error_count = 0;

    for result in results {
        if result.success {
            // Extract exit code number from message "exit code: X"
            let code = result
                .message
                .strip_prefix("exit code: ")
                .unwrap_or(&result.message);
            println!(
                "  {:width$}  {:>4}",
                result.name,
                code,
                width = max_name_len
            );
        } else {
            error_count += 1;
            println!(
                "  {:width$}  ERROR: {}",
                result.name,
                result.message,
                width = max_name_len
            );
        }
    }

    println!();
    if error_count > 0 {
        println!("  Run: {} error(s)", error_count);
    }
    print_hline!();
    println!();

    if error_count > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
