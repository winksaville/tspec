//! Batch operations on all workspace packages
//!
//! Provides build_all, run_all, test_all for operating on all workspace members.

use std::os::unix::fs::PermissionsExt;
use std::process::ExitCode;

use std::path::{Path, PathBuf};

use crate::binary::{binary_size, strip_binary};
use crate::cargo_build::{build_package, test_package};
use crate::compare::{SpecResult, compare_specs, print_comparison};
use crate::find_paths::find_tspecs;
use crate::run::run_binary;
use crate::tspec::spec_name_from_path;
use crate::types::Verbosity;
use crate::workspace::{PackageKind, WorkspaceInfo};
use crate::{print_header, print_hline};

/// Normalize tspec patterns for per-package matching in all-packages mode.
///
/// Strips directory components so shell-expanded paths from the workspace root
/// don't leak into sub-packages. Filters out non-tspec shell expansions
/// (e.g. `target/`, `tools/`) — only `.ts.toml` files and glob patterns are kept.
///
/// Returns `None` if all patterns were filtered out (likely shell expansion of a
/// glob that matched non-tspec entries). The caller should warn about quoting.
fn normalize_tspec_patterns(patterns: &[String]) -> Option<Vec<String>> {
    if patterns.is_empty() {
        return Some(Vec::new());
    }
    let filenames: Vec<String> = patterns
        .iter()
        .filter_map(|p| {
            let name = Path::new(p)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.clone());
            // Keep glob patterns (unexpanded) and actual tspec files
            let is_glob = name.contains('*') || name.contains('?') || name.contains('[');
            let is_tspec = name.ends_with(crate::TSPEC_SUFFIX);
            if is_glob || is_tspec {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    if filenames.is_empty() && !patterns.is_empty() {
        None // all patterns were filtered — likely shell expansion
    } else {
        Some(filenames)
    }
}

/// Warn that shell glob expansion likely ate the tspec pattern.
pub fn warn_shell_glob_expansion(patterns: &[String]) {
    eprintln!(
        "Warning: -t arguments ({}) don't look like tspec files.",
        patterns.join(", ")
    );
    eprintln!("  The shell likely expanded your glob before tspec could see it.");
    eprintln!("  Quote the pattern to prevent shell expansion: -t 'pattern*'");
}

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

/// Extract a short spec label from an optional tspec path.
fn spec_label(tspec: &Option<String>) -> String {
    match tspec {
        Some(path) => spec_name_from_path(Path::new(path)),
        None => String::new(),
    }
}

/// Result of a batch operation on a single package
pub struct OpResult {
    pub name: String,
    pub spec: String,
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
    verbosity: Verbosity,
) -> Vec<OpResult> {
    let normalized = match normalize_tspec_patterns(tspec_patterns) {
        Some(n) => n,
        None => {
            warn_shell_glob_expansion(tspec_patterns);
            return Vec::new();
        }
    };
    let mut results = Vec::new();

    for member in workspace.buildable_members() {
        let specs = resolve_specs_for_member(&member.path, &normalized);
        if specs.is_empty() && !normalized.is_empty() {
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
            let spec = spec_label(tspec);
            let result = match build_package(&member.name, tspec.as_deref(), cli_profile, verbosity)
            {
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
                        spec,
                        success: true,
                        message: format!("{}", build_result.binary_path.display()),
                        size,
                    }
                }
                Err(e) => OpResult {
                    name: member.name.clone(),
                    spec,
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
    verbosity: Verbosity,
) -> Vec<OpResult> {
    let normalized = match normalize_tspec_patterns(tspec_patterns) {
        Some(n) => n,
        None => {
            warn_shell_glob_expansion(tspec_patterns);
            return Vec::new();
        }
    };
    let mut results = Vec::new();

    for member in workspace.runnable_members() {
        let specs = resolve_specs_for_member(&member.path, &normalized);
        if specs.is_empty() && !normalized.is_empty() {
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
            let spec = spec_label(tspec);
            let result = match build_package(&member.name, tspec.as_deref(), cli_profile, verbosity)
            {
                Ok(build_result) => {
                    if strip && let Err(e) = strip_binary(&build_result.binary_path) {
                        eprintln!("  warning: strip failed: {}", e);
                    }
                    match run_binary(&build_result.binary_path, &[]) {
                        Ok(exit_code) => OpResult {
                            name: member.name.clone(),
                            spec: spec.clone(),
                            success: true,
                            message: format!("exit code: {}", exit_code),
                            size: None,
                        },
                        Err(e) => OpResult {
                            name: member.name.clone(),
                            spec: spec.clone(),
                            success: false,
                            message: format!("run failed: {}", e),
                            size: None,
                        },
                    }
                }
                Err(e) => OpResult {
                    name: member.name.clone(),
                    spec,
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
    verbosity: Verbosity,
) -> Vec<OpResult> {
    let normalized = match normalize_tspec_patterns(tspec_patterns) {
        Some(n) => n,
        None => {
            warn_shell_glob_expansion(tspec_patterns);
            return Vec::new();
        }
    };
    let mut results = Vec::new();

    // Test regular packages (excluding Test kind which needs special handling)
    for member in workspace.buildable_members() {
        if member.kind == PackageKind::Test {
            continue; // Handle test packages separately
        }

        let specs = resolve_specs_for_member(&member.path, &normalized);
        if specs.is_empty() && !normalized.is_empty() {
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
            let spec = spec_label(tspec);
            let result = match test_package(&member.name, tspec.as_deref(), cli_profile, verbosity)
            {
                Ok(()) => OpResult {
                    name: member.name.clone(),
                    spec,
                    success: true,
                    message: "ok".to_string(),
                    size: None,
                },
                Err(e) => OpResult {
                    name: member.name.clone(),
                    spec,
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
        let specs = resolve_specs_for_member(&member.path, &normalized);
        if specs.is_empty() && !normalized.is_empty() {
            continue;
        }

        println!("=== {} ===", member.name);

        // Build the test package (builds all binaries)
        let build_tspec = specs.first().map(|p| p.to_string_lossy().into_owned());
        let spec = spec_label(&build_tspec);
        let build_result =
            match build_package(&member.name, build_tspec.as_deref(), cli_profile, verbosity) {
                Ok(r) => r,
                Err(e) => {
                    results.push(OpResult {
                        name: member.name.clone(),
                        spec,
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
                spec: spec.clone(),
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
                            spec: spec.clone(),
                            success: true,
                            message: "ok".to_string(),
                            size: None,
                        }
                    } else {
                        println!("FAILED (exit {})", exit_code);
                        OpResult {
                            name: format!("{}/{}", member.name, bin_name),
                            spec: spec.clone(),
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
                        spec: spec.clone(),
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

/// A row for the summary table: package name, spec, and pre-formatted detail string.
struct SummaryRow {
    name: String,
    spec: String,
    detail: String,
}

/// Print a summary table with consistent formatting across all operations.
///
/// - `ws_name` — workspace/package name for the header
/// - `cmd` — operation name ("TEST", "BUILD", "RUN", "COMPARE")
/// - `detail_header` — column header for the detail column (e.g. "Status", "Status    Size", "Exit")
/// - `rows` — pre-formatted rows
/// - `footer` — footer line (e.g. "Test: 3 passed, 1 failed")
fn print_summary_table(
    ws_name: &str,
    cmd: &str,
    detail_header: &str,
    rows: &[SummaryRow],
    footer: &str,
) {
    let max_name_len = rows.iter().map(|r| r.name.len()).max().unwrap_or(7).max(7);
    let has_spec = rows.iter().any(|r| !r.spec.is_empty());
    let max_spec_len = if has_spec {
        rows.iter().map(|r| r.spec.len()).max().unwrap_or(4).max(4)
    } else {
        0
    };

    println!();
    print_header!(format!("{ws_name} {cmd} SUMMARY"));
    if has_spec {
        println!(
            "  {:nw$}  {:sw$}  {detail_header}",
            "Package",
            "Spec",
            nw = max_name_len,
            sw = max_spec_len
        );
    } else {
        println!(
            "  {:width$}  {detail_header}",
            "Package",
            width = max_name_len
        );
    }

    for row in rows {
        if has_spec {
            println!(
                "  {:nw$}  {:sw$}  {}",
                row.name,
                row.spec,
                row.detail,
                nw = max_name_len,
                sw = max_spec_len
            );
        } else {
            println!(
                "  {:width$}  {}",
                row.name,
                row.detail,
                width = max_name_len
            );
        }
    }

    println!();
    if !footer.is_empty() {
        println!("  {footer}");
    }
    print_hline!();
    println!();
}

/// Print a summary of operation results (for tests)
pub fn print_test_summary(name: &str, results: &[OpResult]) -> ExitCode {
    let mut passed = 0;
    let mut failed = 0;

    let rows: Vec<SummaryRow> = results
        .iter()
        .map(|r| {
            let detail = if r.success {
                passed += 1;
                "[PASS]".to_string()
            } else {
                failed += 1;
                "[FAIL]".to_string()
            };
            SummaryRow {
                name: r.name.clone(),
                spec: r.spec.clone(),
                detail,
            }
        })
        .collect();

    print_summary_table(
        name,
        "TEST",
        "Status",
        &rows,
        &format!("Test: {passed} passed, {failed} failed"),
    );

    if failed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Print a summary for build operations (OK/FAILED)
pub fn print_summary(name: &str, results: &[OpResult]) -> ExitCode {
    let mut ok_count = 0;
    let mut failed_count = 0;

    let rows: Vec<SummaryRow> = results
        .iter()
        .map(|r| {
            let status = if r.success {
                ok_count += 1;
                "[ OK ]"
            } else {
                failed_count += 1;
                "[FAIL]"
            };
            let size_str = r.size.map(format_size).unwrap_or_else(|| "--".to_string());
            SummaryRow {
                name: r.name.clone(),
                spec: r.spec.clone(),
                detail: format!("{status}  {size_str:>6}"),
            }
        })
        .collect();

    print_summary_table(
        name,
        "BUILD",
        "Status    Size",
        &rows,
        &format!("Build: {ok_count} ok, {failed_count} failed"),
    );

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
    verbosity: Verbosity,
) -> Vec<CompareResult> {
    let normalized = match normalize_tspec_patterns(tspec_patterns) {
        Some(n) => n,
        None => {
            warn_shell_glob_expansion(tspec_patterns);
            return Vec::new();
        }
    };
    let mut results = Vec::new();

    for member in workspace.buildable_members() {
        if !member.has_binary {
            continue;
        }

        let spec_paths = if normalized.is_empty() {
            find_tspecs(&member.path, &[]).unwrap_or_default()
        } else {
            let resolved = resolve_specs_for_member(&member.path, &normalized);
            if resolved.is_empty() {
                continue;
            }
            resolved
        };

        println!("=== {} ===", member.name);

        let (op, specs) = match compare_specs(&member.name, &spec_paths, verbosity) {
            Ok(spec_results) => (
                OpResult {
                    name: member.name.clone(),
                    spec: String::new(),
                    success: true,
                    message: "ok".to_string(),
                    size: None,
                },
                spec_results,
            ),
            Err(e) => (
                OpResult {
                    name: member.name.clone(),
                    spec: String::new(),
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
pub fn print_compare_summary(name: &str, results: &[CompareResult]) -> ExitCode {
    let has_failure = results.iter().any(|r| !r.op.success);

    // Reprint per-package comparison tables together
    for result in results {
        if !result.specs.is_empty() {
            print_comparison(&result.op.name, &result.specs);
        }
    }

    // Only show overall COMPARE SUMMARY when there are multiple packages
    if results.len() > 1 {
        let mut ok_count = 0;
        let mut failed_count = 0;

        let rows: Vec<SummaryRow> = results
            .iter()
            .map(|r| {
                let detail = if r.op.success {
                    ok_count += 1;
                    "[ OK ]".to_string()
                } else {
                    failed_count += 1;
                    "[FAIL]".to_string()
                };
                SummaryRow {
                    name: r.op.name.clone(),
                    spec: r.op.spec.clone(),
                    detail,
                }
            })
            .collect();

        print_summary_table(
            name,
            "COMPARE",
            "Status",
            &rows,
            &format!("Compare: {ok_count} ok, {failed_count} failed"),
        );
    }

    if has_failure {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Print a summary for run operations (shows exit codes, not pass/fail)
pub fn print_run_summary(name: &str, results: &[OpResult]) -> ExitCode {
    let mut error_count = 0;

    let rows: Vec<SummaryRow> = results
        .iter()
        .map(|r| {
            let detail = if r.success {
                let code = r.message.strip_prefix("exit code: ").unwrap_or(&r.message);
                format!("{code:>4}")
            } else {
                error_count += 1;
                format!("ERROR: {}", r.message)
            };
            SummaryRow {
                name: r.name.clone(),
                spec: r.spec.clone(),
                detail,
            }
        })
        .collect();

    let footer = if error_count > 0 {
        format!("Run: {error_count} error(s)")
    } else {
        String::new()
    };

    print_summary_table(name, "RUN", "Exit", &rows, &footer);

    if error_count > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
