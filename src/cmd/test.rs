use anyhow::{Context, Result};
use clap::Args;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, current_package_name, resolve_package_arg};
use crate::all::{print_test_summary, test_all};
use crate::cargo_build::test_package;
use crate::find_paths::{find_tspecs, get_package_name, resolve_package_dir};
use crate::types::CargoFlags;
use crate::workspace::WorkspaceInfo;

/// Parsed test statistics from cargo test output.
#[derive(Debug, Default, Clone)]
pub struct TestResult {
    pub passed: u32,
    pub failed: u32,
    pub ignored: u32,
    pub filtered: u32,
}

impl TestResult {
    /// Aggregate another TestResult into this one.
    pub fn merge(&mut self, other: &TestResult) {
        self.passed += other.passed;
        self.failed += other.failed;
        self.ignored += other.ignored;
        self.filtered += other.filtered;
    }

    /// Total tests that actually ran (passed + failed).
    pub fn total_ran(&self) -> u32 {
        self.passed + self.failed
    }
}

/// Parse a `test result:` line from cargo test output.
///
/// Example: `test result: ok. 263 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s`
pub fn parse_test_result_line(line: &str) -> Option<TestResult> {
    let rest = line.strip_prefix("test result:")?;
    let rest = rest.trim();
    // Skip the "ok." or "FAILED." prefix
    let rest = rest
        .strip_prefix("ok.")
        .or_else(|| rest.strip_prefix("FAILED."))?;
    let rest = rest.trim();

    let mut result = TestResult::default();
    for part in rest.split(';') {
        let part = part.trim();
        if let Some((num, label)) = part.split_once(' ') {
            let Ok(n) = num.parse::<u32>() else {
                continue;
            };
            match label.trim() {
                "passed" => result.passed = n,
                "failed" => result.failed = n,
                "ignored" => result.ignored = n,
                "filtered" | "filtered out" => result.filtered = n,
                _ => {} // "measured"
            }
        }
    }
    Some(result)
}

/// Parse and aggregate raw `test result:` lines into a single TestResult.
pub fn parse_test_results(lines: &[String]) -> TestResult {
    let mut aggregated = TestResult::default();
    for line in lines {
        if let Some(parsed) = parse_test_result_line(line) {
            aggregated.merge(&parsed);
        }
    }
    aggregated
}

/// Test package(s) with a translation spec
#[derive(Args)]
pub struct TestCmd {
    /// Package to test (name or path, e.g. "." for current dir)
    #[arg(value_name = "PACKAGE")]
    pub positional: Option<String>,
    /// Package to test (defaults to current directory or all packages)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Test all workspace packages (even when in a package directory)
    #[arg(short = 'w', long = "workspace")]
    pub workspace: bool,
    /// Spec file(s) or glob pattern(s) to test with (defaults to package's tspec file)
    #[arg(short = 't', long = "tspec", num_args = 1..)]
    pub tspec: Vec<String>,
    /// Release build
    #[arg(short, long, conflicts_with = "profile")]
    pub release: bool,
    /// Build profile (e.g., release, release-small, or any custom profile)
    #[arg(long)]
    pub profile: Option<String>,
    /// Stop on first failure
    #[arg(short, long)]
    pub fail_fast: bool,
    /// List test targets and test functions
    #[arg(short, long)]
    pub list: bool,
    /// List available target names for use with --test
    #[arg(long = "target-names")]
    pub target_names: bool,
    /// Filter by qualified test name, e.g. '-n _by_' matches 'ts_cmd::remove::tests::remove_by_index'
    #[arg(short = 'n', long = "names", num_args = 1..)]
    pub name_filter: Vec<String>,
    /// Run only the named test target (use --target-names to see available names; repeatable)
    #[arg(long = "test", value_name = "TARGET-NAME")]
    pub test_target: Vec<String>,
    /// Include #[ignore]'d tests (equivalent to -- --include-ignored)
    #[arg(long = "all-tests")]
    pub all_tests: bool,
    /// Extra arguments passed after -- to the test binary (e.g., --ignored)
    #[arg(last = true)]
    pub test_args: Vec<String>,
}

impl TestCmd {
    fn effective_profile(&self) -> Option<&str> {
        if let Some(ref p) = self.profile {
            Some(p)
        } else if self.release {
            Some("release")
        } else {
            None
        }
    }
}

impl Execute for TestCmd {
    fn execute(&self, project_root: &Path, flags: &CargoFlags) -> Result<ExitCode> {
        let cli_profile = self.effective_profile();

        // Resolve package: --workspace > -p/positional PKG > cwd > all
        let resolved = if self.workspace {
            None
        } else {
            match self.positional.as_deref().or(self.package.as_deref()) {
                Some(pkg) => resolve_package_arg(pkg, project_root)?,
                None => current_package_name(project_root),
            }
        };

        if self.target_names {
            return list_target_names(resolved.as_deref(), project_root, flags);
        }

        if self.list {
            return list_tests(resolved.as_deref(), &self.name_filter, project_root, flags);
        }

        // Build extra_args from --names, --test, --all-tests, and trailing args
        let has_extra = !self.name_filter.is_empty()
            || !self.test_target.is_empty()
            || !self.test_args.is_empty()
            || self.all_tests;
        let flags = if has_extra {
            let mut f = flags.clone();
            for target in &self.test_target {
                f.extra_args.push("--test".to_string());
                f.extra_args.push(target.clone());
            }
            // Name filters, --all-tests, and test_args all go after --
            // The test harness accepts [FILTERS...] as OR-matched substrings
            if !self.name_filter.is_empty() || !self.test_args.is_empty() || self.all_tests {
                f.extra_args.push("--".to_string());
                f.extra_args.extend(self.name_filter.clone());
                if self.all_tests {
                    f.extra_args.push("--include-ignored".to_string());
                }
                f.extra_args.extend(self.test_args.clone());
            }
            f
        } else {
            flags.clone()
        };

        match resolved {
            None => {
                let workspace = WorkspaceInfo::discover(project_root)?;

                // When --test targets are specified in workspace mode, only test
                // packages that actually have matching files in tests/
                if !self.test_target.is_empty() {
                    let members = workspace.buildable_members();
                    let matching: Vec<_> = members
                        .iter()
                        .filter(|m| {
                            self.test_target
                                .iter()
                                .any(|t| m.path.join("tests").join(format!("{t}.rs")).exists())
                        })
                        .collect();

                    if matching.is_empty() {
                        eprintln!(
                            "No packages contain test targets: {}",
                            self.test_target.join(", ")
                        );
                        return Ok(ExitCode::from(1));
                    }

                    for member in &matching {
                        println!("=== {} ===", member.name);
                        test_package(&member.name, None, cli_profile, project_root, &flags)?;
                    }
                    return Ok(ExitCode::SUCCESS);
                }

                let results =
                    test_all(&workspace, &self.tspec, cli_profile, self.fail_fast, &flags);
                Ok(print_test_summary(&workspace.name_versioned(), &results))
            }
            Some(name) => {
                let mut all_lines = Vec::new();
                if self.tspec.is_empty() {
                    all_lines = test_package(&name, None, cli_profile, project_root, &flags)?;
                } else {
                    let package_dir = resolve_package_dir(project_root, Some(&name))?;
                    let pkg_name = get_package_name(&package_dir)?;
                    let spec_paths = find_tspecs(&package_dir, &self.tspec)?;
                    for spec_path in &spec_paths {
                        let spec_str = spec_path.to_string_lossy();
                        let lines = test_package(
                            &pkg_name,
                            Some(&spec_str),
                            cli_profile,
                            project_root,
                            &flags,
                        )?;
                        all_lines.extend(lines);
                    }
                }
                let counts = parse_test_results(&all_lines);
                if counts.total_ran() == 0 {
                    eprintln!("Error: 0 tests ran (filter may not match anything)");
                    return Ok(ExitCode::from(1));
                }
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

/// Format a cargo test target header into a human-friendly label.
///
/// Cargo emits headers like:
/// - `unittests src/lib.rs (target/debug/deps/tspec-abc123)` → unit tests (no --test name)
/// - `tests/tspec_test.rs (target/debug/deps/tspec_test-abc123)` → integration test
///
/// For integration tests, we show the `--test` usable name prominently.
fn format_target_header(raw: &str) -> String {
    // Strip the "(target/...)" suffix for cleaner output
    let label = if let Some(idx) = raw.find(" (") {
        &raw[..idx]
    } else {
        raw
    };

    if label.starts_with("unittests ") {
        // "unittests src/lib.rs" → unit tests, not selectable with --test
        format!("unit tests ({})", label.strip_prefix("unittests ").unwrap())
    } else if label.starts_with("tests/") {
        // "tests/tspec_test.rs" → extract basename for --test
        let basename = label
            .strip_prefix("tests/")
            .and_then(|s| s.strip_suffix(".rs"))
            .unwrap_or(label);
        format!("{} (--test {})", label, basename)
    } else {
        label.to_string()
    }
}

/// List available target names for `--test`.
///
/// Runs `cargo test --no-run` and parses stderr for "Running tests/..." lines,
/// extracting the basename (without .rs) as the target name.
fn list_target_names(
    package: Option<&str>,
    project_root: &Path,
    flags: &CargoFlags,
) -> Result<ExitCode> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("test").arg("--no-run");
    if let Some(pkg) = package {
        cmd.arg("-p").arg(pkg);
    }
    flags.apply_to_command(&mut cmd);
    cmd.current_dir(project_root);

    let output = cmd.output().context("failed to run cargo test --no-run")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{}", stderr);
        return Ok(ExitCode::from(1));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut found = false;

    println!("Available target names for --test:");
    for line in stderr.lines() {
        let trimmed = line.trim();
        // --no-run emits "Executable tests/foo.rs (target/...)" for integration tests
        if let Some(rest) = trimmed
            .strip_prefix("Executable tests/")
            .or_else(|| trimmed.strip_prefix("Running tests/"))
        {
            let file = if let Some(idx) = rest.find(" (") {
                &rest[..idx]
            } else {
                rest
            };
            if let Some(name) = file.strip_suffix(".rs") {
                println!("  {}    (tests/{})", name, file);
                found = true;
            }
        }
    }

    if !found {
        println!("  (none — no integration test files in tests/)");
    }

    Ok(ExitCode::SUCCESS)
}

/// Run `cargo test -- --list` and format the output.
///
/// Groups test functions under their target headers, showing counts per target
/// and a total. Skips targets with zero tests.
fn list_tests(
    package: Option<&str>,
    name_filter: &[String],
    project_root: &Path,
    flags: &CargoFlags,
) -> Result<ExitCode> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("test");
    if let Some(pkg) = package {
        cmd.arg("-p").arg(pkg);
    }
    flags.apply_to_command(&mut cmd);
    cmd.arg("--");
    cmd.args(name_filter);
    cmd.arg("--list");
    cmd.current_dir(project_root);

    let output = cmd.output().context("failed to run cargo test -- --list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{}", stderr);
        return Ok(ExitCode::from(1));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse target headers from stderr ("Running ..." and "Doc-tests ..." lines)
    // Produce human-friendly labels that show the --test name when applicable.
    let targets: Vec<String> = stderr
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("Running ") {
                let rest = trimmed.strip_prefix("Running ").unwrap();
                Some(format_target_header(rest))
            } else if trimmed.starts_with("Doc-tests ") {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .collect();

    // Parse stdout into groups split by "N tests, M benchmarks" summary lines
    let mut groups: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in stdout.lines() {
        if line.contains("tests,") && line.contains("benchmarks") {
            groups.push(std::mem::take(&mut current));
        } else if line.ends_with(": test") || line.ends_with(": bench") {
            current.push(line);
        }
    }

    // Print grouped output
    let mut total = 0;
    for (i, tests) in groups.iter().enumerate() {
        if tests.is_empty() {
            continue;
        }
        let unknown = "unknown target".to_string();
        let header = targets.get(i).unwrap_or(&unknown);
        println!("{} ({} tests)", header, tests.len());
        for test in tests {
            // Strip ": test" / ": bench" suffix for cleaner output
            let name = test
                .strip_suffix(": test")
                .or_else(|| test.strip_suffix(": bench"))
                .unwrap_or(test);
            println!("  {}", name);
        }
        total += tests.len();
        println!();
    }

    println!("{} tests total", total);
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    fn parse(args: &[&str]) -> TestCmd {
        let mut full = vec!["tspec", "test"];
        full.extend_from_slice(args);
        let cli = Cli::try_parse_from(full).unwrap();
        match cli.command {
            crate::cli::Commands::Test(cmd) => cmd,
            _ => panic!("expected Test command"),
        }
    }

    #[test]
    fn package_optional() {
        let cmd = parse(&[]);
        assert!(cmd.package.is_none());
    }

    #[test]
    fn package_explicit() {
        let cmd = parse(&["-p", "myapp"]);
        assert_eq!(cmd.package.as_deref(), Some("myapp"));
    }

    #[test]
    fn tspec_empty_by_default() {
        let cmd = parse(&[]);
        assert!(cmd.tspec.is_empty());
    }

    #[test]
    fn tspec_single_file() {
        let cmd = parse(&["-t", "foo.ts.toml"]);
        assert_eq!(cmd.tspec, vec!["foo.ts.toml"]);
    }

    #[test]
    fn tspec_multiple_values_one_flag() {
        let cmd = parse(&["-t", "a.ts.toml", "b.ts.toml"]);
        assert_eq!(cmd.tspec, vec!["a.ts.toml", "b.ts.toml"]);
    }

    #[test]
    fn tspec_multiple_flags() {
        let cmd = parse(&["-t", "a.ts.toml", "-t", "b.ts.toml"]);
        assert_eq!(cmd.tspec, vec!["a.ts.toml", "b.ts.toml"]);
    }

    #[test]
    fn all_flags_together() {
        let cmd = parse(&["-p", "myapp", "-t", "spec.ts.toml"]);
        assert_eq!(cmd.package.as_deref(), Some("myapp"));
        assert_eq!(cmd.tspec, vec!["spec.ts.toml"]);
    }

    #[test]
    fn workspace_flag_short() {
        let cmd = parse(&["-w"]);
        assert!(cmd.workspace);
    }

    #[test]
    fn workspace_flag_long() {
        let cmd = parse(&["--workspace"]);
        assert!(cmd.workspace);
    }

    #[test]
    fn workspace_default_false() {
        let cmd = parse(&[]);
        assert!(!cmd.workspace);
    }

    #[test]
    fn fail_fast_flag() {
        let cmd = parse(&["-w", "-f"]);
        assert!(cmd.workspace);
        assert!(cmd.fail_fast);
    }

    #[test]
    fn fail_fast_long() {
        let cmd = parse(&["--workspace", "--fail-fast"]);
        assert!(cmd.workspace);
        assert!(cmd.fail_fast);
    }

    #[test]
    fn release_flag() {
        let cmd = parse(&["-r"]);
        assert!(cmd.release);
    }

    #[test]
    fn profile_flag() {
        let cmd = parse(&["--profile", "release-small"]);
        assert_eq!(cmd.profile.as_deref(), Some("release-small"));
        assert_eq!(cmd.effective_profile(), Some("release-small"));
    }

    #[test]
    fn profile_and_release_conflict() {
        let result = Cli::try_parse_from(["tspec", "test", "-r", "--profile", "custom"]);
        assert!(result.is_err());
    }

    #[test]
    fn target_names_flag() {
        let cmd = parse(&["--target-names"]);
        assert!(cmd.target_names);
    }

    #[test]
    fn name_filter_single() {
        let cmd = parse(&["-n", "my_test"]);
        assert_eq!(cmd.name_filter, vec!["my_test"]);
    }

    #[test]
    fn name_filter_long() {
        let cmd = parse(&["--names", "my_test"]);
        assert_eq!(cmd.name_filter, vec!["my_test"]);
    }

    #[test]
    fn name_filter_multiple() {
        let cmd = parse(&["-n", "foo", "bar"]);
        assert_eq!(cmd.name_filter, vec!["foo", "bar"]);
    }

    #[test]
    fn name_filter_with_package() {
        let cmd = parse(&["-p", "myapp", "-n", "spec_default"]);
        assert_eq!(cmd.package.as_deref(), Some("myapp"));
        assert_eq!(cmd.name_filter, vec!["spec_default"]);
    }

    #[test]
    fn test_target_flag() {
        let cmd = parse(&["--test", "integration_test"]);
        assert_eq!(cmd.test_target, vec!["integration_test"]);
    }

    #[test]
    fn test_target_multiple() {
        let cmd = parse(&["--test", "foo", "--test", "bar"]);
        assert_eq!(cmd.test_target, vec!["foo", "bar"]);
    }

    #[test]
    fn trailing_args() {
        let cmd = parse(&["--", "--ignored"]);
        assert_eq!(cmd.test_args, vec!["--ignored"]);
    }

    #[test]
    fn test_target_and_trailing_args() {
        let cmd = parse(&[
            "--test",
            "integration_test",
            "--",
            "--ignored",
            "--nocapture",
        ]);
        assert_eq!(cmd.test_target, vec!["integration_test"]);
        assert_eq!(cmd.test_args, vec!["--ignored", "--nocapture"]);
    }

    #[test]
    fn test_target_with_package() {
        let cmd = parse(&[
            "-p",
            "tspec",
            "--test",
            "integration_test",
            "--",
            "--ignored",
        ]);
        assert_eq!(cmd.package.as_deref(), Some("tspec"));
        assert_eq!(cmd.test_target, vec!["integration_test"]);
        assert_eq!(cmd.test_args, vec!["--ignored"]);
    }

    #[test]
    fn all_tests_flag() {
        let cmd = parse(&["--all-tests"]);
        assert!(cmd.all_tests);
    }

    #[test]
    fn all_tests_default_false() {
        let cmd = parse(&[]);
        assert!(!cmd.all_tests);
    }

    #[test]
    fn format_header_unit_tests() {
        let h = format_target_header("unittests src/lib.rs (target/debug/deps/tspec-abc123)");
        assert_eq!(h, "unit tests (src/lib.rs)");
    }

    #[test]
    fn format_header_integration_test() {
        let h = format_target_header("tests/tspec_test.rs (target/debug/deps/tspec_test-abc123)");
        assert_eq!(h, "tests/tspec_test.rs (--test tspec_test)");
    }

    #[test]
    fn format_header_unknown() {
        let h = format_target_header("something_else");
        assert_eq!(h, "something_else");
    }

    // parse_test_result_line unit tests

    #[test]
    fn parse_test_result_ok() {
        let line = "test result: ok. 263 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s";
        let r = parse_test_result_line(line).unwrap();
        assert_eq!(r.passed, 263);
        assert_eq!(r.failed, 0);
        assert_eq!(r.ignored, 0);
        assert_eq!(r.filtered, 0);
    }

    #[test]
    fn parse_test_result_failed() {
        let line = "test result: FAILED. 10 passed; 2 failed; 1 ignored; 0 measured; 5 filtered out; finished in 1.23s";
        let r = parse_test_result_line(line).unwrap();
        assert_eq!(r.passed, 10);
        assert_eq!(r.failed, 2);
        assert_eq!(r.ignored, 1);
        assert_eq!(r.filtered, 5);
    }

    #[test]
    fn parse_test_result_all_zeros() {
        let line = "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s";
        let r = parse_test_result_line(line).unwrap();
        assert_eq!(r.passed, 0);
        assert_eq!(r.failed, 0);
        assert_eq!(r.total_ran(), 0);
    }

    #[test]
    fn parse_test_result_not_a_result_line() {
        assert!(parse_test_result_line("running 42 tests").is_none());
        assert!(parse_test_result_line("test foo::bar ... ok").is_none());
        assert!(parse_test_result_line("").is_none());
    }

    #[test]
    fn test_result_merge() {
        let mut a = TestResult {
            passed: 10,
            failed: 1,
            ignored: 2,
            filtered: 3,
        };
        let b = TestResult {
            passed: 5,
            failed: 0,
            ignored: 1,
            filtered: 2,
        };
        a.merge(&b);
        assert_eq!(a.passed, 15);
        assert_eq!(a.failed, 1);
        assert_eq!(a.ignored, 3);
        assert_eq!(a.filtered, 5);
    }

    #[test]
    fn parse_test_results_aggregates() {
        let lines = vec![
            "test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.1s".to_string(),
            "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.2s".to_string(),
        ];
        let r = parse_test_results(&lines);
        assert_eq!(r.passed, 15);
        assert_eq!(r.failed, 0);
        assert_eq!(r.ignored, 1);
        assert_eq!(r.filtered, 3);
    }
}
