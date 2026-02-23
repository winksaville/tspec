use anyhow::{Context, Result, bail};
use glob::Pattern;
use std::path::{Path, PathBuf};

use crate::TSPEC_SUFFIX;
use crate::types::{Spec, profile_dir_name};

/// Check if TOML content has an exact section header like `[workspace]` or `[package]`
/// Only matches lines where the trimmed content equals `[section]` exactly
fn has_toml_section_exact(content: &str, section: &str) -> bool {
    let header = format!("[{}]", section);
    content.lines().any(|line| line.trim() == header)
}

/// Extract package name from Cargo.toml
pub fn get_package_name(crate_dir: &Path) -> Result<String> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml)
        .with_context(|| format!("failed to read {}", cargo_toml.display()))?;

    // Simple parsing - look for name = "..." in [package] section
    let mut in_package = false;
    for line in content.lines() {
        let line = line.trim();
        if line == "[package]" {
            in_package = true;
            continue;
        }
        if line.starts_with('[') {
            in_package = false;
            continue;
        }
        if in_package
            && line.starts_with("name")
            && let Some(eq_pos) = line.find('=')
        {
            let value = line[eq_pos + 1..].trim();
            let value = value.trim_matches('"').trim_matches('\'');
            return Ok(value.to_string());
        }
    }
    bail!("could not find package name in {}", cargo_toml.display());
}

/// Find the project root by looking for Cargo.toml with [workspace] or [package]
/// For workspaces, returns the directory containing the workspace Cargo.toml
/// For POPs (Plain Old Packages), returns the directory containing the package Cargo.toml
/// Respects workspace `exclude` — a package inside an excluded path is treated as a POP.
pub fn find_project_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    let mut package_root: Option<PathBuf> = None;

    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            // Workspace takes precedence — unless the package we found is excluded
            if has_toml_section_exact(&content, "workspace") {
                if let Some(ref pkg_root) = package_root
                    && is_excluded_from_workspace(&content, &dir, pkg_root)
                {
                    return Ok(pkg_root.clone());
                }
                return Ok(dir);
            }
            // Remember the first (deepest) package we find as potential POP root
            if package_root.is_none() && has_toml_section_exact(&content, "package") {
                package_root = Some(dir.clone());
            }
        }
        if !dir.pop() {
            // No workspace found, use the POP root if we found one
            if let Some(root) = package_root {
                return Ok(root);
            }
            bail!("could not find project root (no Cargo.toml with [workspace] or [package])");
        }
    }
}

/// Resolve a `--manifest-path` argument to a project root.
/// Accepts a path to a Cargo.toml file or a directory containing one.
/// Walks up from the resolved directory to find the workspace root,
/// reusing the same logic as `find_project_root()`.
pub fn resolve_manifest_path(path: &Path) -> Result<PathBuf> {
    // Canonicalize so walk-up works on relative paths
    let canon = path
        .canonicalize()
        .with_context(|| format!("path not found: {}", path.display()))?;
    let start_dir = if canon.is_file() {
        canon
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid manifest path: {}", path.display()))?
            .to_path_buf()
    } else {
        canon
    };

    // Verify Cargo.toml exists at the starting directory
    if !start_dir.join("Cargo.toml").exists() {
        bail!("no Cargo.toml found at {}", start_dir.display());
    }

    // Walk up to find workspace root, same logic as find_project_root()
    let mut dir = start_dir.clone();
    let mut package_root: Option<PathBuf> = None;

    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            if has_toml_section_exact(&content, "workspace") {
                if let Some(ref pkg_root) = package_root
                    && is_excluded_from_workspace(&content, &dir, pkg_root)
                {
                    return Ok(pkg_root.clone());
                }
                return Ok(dir);
            }
            if package_root.is_none() && has_toml_section_exact(&content, "package") {
                package_root = Some(dir.clone());
            }
        }
        if !dir.pop() {
            if let Some(root) = package_root {
                return Ok(root);
            }
            bail!("no project root found from {}", start_dir.display());
        }
    }
}

/// Check if a package directory is excluded from a workspace.
/// `ws_content` is the workspace Cargo.toml content, `ws_dir` is the workspace root,
/// and `pkg_dir` is the package directory to check.
fn is_excluded_from_workspace(ws_content: &str, ws_dir: &Path, pkg_dir: &Path) -> bool {
    let Ok(rel) = pkg_dir.strip_prefix(ws_dir) else {
        return false;
    };
    let rel_str = rel.to_string_lossy();

    // Parse workspace.exclude from the TOML
    let Ok(doc) = ws_content.parse::<toml::Value>() else {
        return false;
    };
    let Some(excludes) = doc
        .get("workspace")
        .and_then(|w| w.get("exclude"))
        .and_then(|e| e.as_array())
    else {
        return false;
    };

    for exclude in excludes {
        let Some(pattern) = exclude.as_str() else {
            continue;
        };
        // Check if the package path starts with the exclude pattern
        // e.g., exclude = ["tests/fixtures"] should match "tests/fixtures/pop"
        if rel_str == pattern || rel_str.starts_with(&format!("{}/", pattern)) {
            return true;
        }
        // Also try glob matching
        if let Ok(glob) = Pattern::new(pattern)
            && glob.matches(&rel_str)
        {
            return true;
        }
    }

    false
}

/// Check if a project root is a POP (Plain Old Package) vs a workspace
pub fn is_pop(project_root: &Path) -> bool {
    let cargo_toml = project_root.join("Cargo.toml");
    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        // POP has [package] but no [workspace]
        has_toml_section_exact(&content, "package")
            && !has_toml_section_exact(&content, "workspace")
    } else {
        false
    }
}

/// Resolve package directory from optional name, defaulting to current directory
/// If package is None, uses current directory (must contain Cargo.toml)
/// If package is Some, looks up the package by name
pub fn resolve_package_dir(workspace: &Path, package: Option<&str>) -> Result<PathBuf> {
    match package {
        Some(name) => find_package_dir(workspace, name),
        None => {
            let cwd = std::env::current_dir()?;
            if cwd.join("Cargo.toml").exists() {
                Ok(cwd)
            } else {
                bail!(
                    "not in a package directory (no Cargo.toml found). Use -p to specify a package."
                )
            }
        }
    }
}

/// Find a package's directory - tries as path first, then searches standard locations
/// For POPs, checks if name matches the root package
pub fn find_package_dir(project_root: &Path, name: &str) -> Result<PathBuf> {
    // Try as path first (relative or absolute)
    let as_path = PathBuf::from(name);
    if as_path.join("Cargo.toml").exists() {
        return Ok(as_path.canonicalize().unwrap_or(as_path));
    }

    // Check if name matches the root package
    if let Ok(pkg_name) = get_package_name(project_root)
        && pkg_name == name
    {
        return Ok(project_root.to_path_buf());
    }

    // For POPs, nothing else to search
    if is_pop(project_root) {
        bail!(
            "package '{}' not found (this is a single-package project with package '{}')",
            name,
            get_package_name(project_root).unwrap_or_else(|_| "unknown".to_string())
        );
    }

    // Workspace: search root-level members, then libs/, apps/, tools/
    let root_path = project_root.join(name);
    if root_path.join("Cargo.toml").exists() {
        return Ok(root_path);
    }

    for prefix in ["libs", "apps", "tools"] {
        let path = project_root.join(prefix).join(name);
        if path.join("Cargo.toml").exists() {
            return Ok(path);
        }
    }

    // Special case: nested test crates like libs/rlibc-x2/tests
    for prefix in ["libs", "apps"] {
        for entry in std::fs::read_dir(project_root.join(prefix))
            .into_iter()
            .flatten()
            .flatten()
        {
            let nested = entry.path().join("tests");
            if nested.join("Cargo.toml").exists()
                && let Ok(pkg_name) = get_package_name(&nested)
                && pkg_name == name
            {
                return Ok(nested);
            }
        }
    }

    bail!("package '{}' not found", name);
}

/// Find the tspec for a package - tries as path first, then relative to pkg_dir
/// Returns None if no tspec exists (plain cargo build will be used)
pub fn find_tspec(pkg_dir: &Path, explicit: Option<&str>) -> Result<Option<PathBuf>> {
    match explicit {
        Some(name) => {
            // Try as path first (relative to cwd or absolute)
            let as_path = PathBuf::from(name);
            if as_path.exists() {
                return Ok(Some(as_path.canonicalize().unwrap_or(as_path)));
            }

            // Fallback: relative to package directory
            let in_pkg = pkg_dir.join(name);
            if in_pkg.exists() {
                return Ok(Some(in_pkg));
            }

            // Try with TSPEC_SUFFIX if name has no extension
            if !name.contains('.') {
                let with_suffix = pkg_dir.join(format!("{}{}", name, TSPEC_SUFFIX));
                if with_suffix.exists() {
                    return Ok(Some(with_suffix));
                }
            }

            bail!("tspec not found: {}", name);
        }
        None => {
            let default = pkg_dir.join(format!("tspec{}", TSPEC_SUFFIX));
            if default.exists() {
                Ok(Some(default))
            } else {
                Ok(None) // No tspec = plain cargo build
            }
        }
    }
}

/// Find multiple tspecs by glob patterns
/// If no patterns given, defaults to "tspec*{TSPEC_SUFFIX}"
/// Returns sorted list of paths, errors if none found
pub fn find_tspecs(pkg_dir: &Path, patterns: &[String]) -> Result<Vec<PathBuf>> {
    let default_pattern = format!("tspec*{}", TSPEC_SUFFIX);
    let patterns: Vec<&str> = if patterns.is_empty() {
        vec![&default_pattern]
    } else {
        patterns.iter().map(|s| s.as_str()).collect()
    };

    let mut results = Vec::new();

    for pattern in &patterns {
        // Try as literal path first (relative to cwd or absolute)
        let as_path = PathBuf::from(pattern);
        if as_path.exists() && as_path.is_file() {
            results.push(as_path.canonicalize().unwrap_or(as_path));
            continue;
        }

        // Try as literal path relative to pkg_dir
        let in_pkg = pkg_dir.join(pattern);
        if in_pkg.exists() && in_pkg.is_file() {
            results.push(in_pkg);
            continue;
        }

        // Try as glob pattern in pkg_dir
        let glob_pattern =
            Pattern::new(pattern).with_context(|| format!("invalid glob pattern: {}", pattern))?;

        let entries: Vec<_> = std::fs::read_dir(pkg_dir)
            .with_context(|| format!("cannot read directory: {}", pkg_dir.display()))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                e.path().is_file() && glob_pattern.matches(&name)
            })
            .map(|e| e.path())
            .collect();

        results.extend(entries);
    }

    // Remove duplicates and sort
    results.sort();
    results.dedup();

    if results.is_empty() {
        let pattern_list = patterns.join(", ");
        bail!(
            "no tspec files found matching '{}' in {}",
            pattern_list,
            pkg_dir.display()
        );
    }

    Ok(results)
}

/// Get binary path for a build with a spec.
/// `cli_profile` is the CLI-specified profile (None = debug default).
pub fn get_binary_path(
    workspace: &Path,
    crate_name: &str,
    spec: &Spec,
    cli_profile: Option<&str>,
    expanded_target_dir: Option<&str>,
) -> PathBuf {
    let target = spec.cargo.target_triple.clone().or_else(|| {
        spec.cargo
            .target_json
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
    });

    // Determine profile directory: spec profile takes precedence, then CLI profile
    let dir = match spec.cargo.profile.as_deref() {
        Some(p) => profile_dir_name(p),
        None => match cli_profile {
            Some(p) => profile_dir_name(p),
            None => "debug",
        },
    };

    let base = match expanded_target_dir {
        Some(td) => workspace.join("target").join(td),
        None => workspace.join("target"),
    };

    match target {
        Some(t) => base.join(t).join(dir).join(crate_name),
        None => base.join(dir).join(crate_name),
    }
}

/// Get binary path for a simple build (no spec).
/// `cli_profile` is the CLI-specified profile (None = debug default).
pub fn get_binary_path_simple(
    workspace: &Path,
    crate_name: &str,
    cli_profile: Option<&str>,
) -> PathBuf {
    let dir = match cli_profile {
        Some(p) => profile_dir_name(p),
        None => "debug",
    };
    workspace.join("target").join(dir).join(crate_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_constants::SUFFIX;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // ==================== get_package_name tests ====================

    #[test]
    fn get_package_name_valid() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "my-test-crate"
version = "0.1.0"
"#,
        )
        .unwrap();

        let name = get_package_name(tmp.path()).unwrap();
        assert_eq!(name, "my-test-crate");
    }

    #[test]
    fn get_package_name_with_single_quotes() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            "[package]\nname = 'single-quoted'\nversion = '0.1.0'\n",
        )
        .unwrap();

        let name = get_package_name(tmp.path()).unwrap();
        assert_eq!(name, "single-quoted");
    }

    #[test]
    fn get_package_name_missing_name() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = tmp.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nversion = \"0.1.0\"\n").unwrap();

        let result = get_package_name(tmp.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("could not find package name")
        );
    }

    #[test]
    fn get_package_name_missing_file() {
        let tmp = TempDir::new().unwrap();
        let result = get_package_name(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed to read"));
    }

    #[test]
    fn get_package_name_name_in_other_section_ignored() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[dependencies]
name = "wrong-section"

[package]
name = "correct-name"
version = "0.1.0"
"#,
        )
        .unwrap();

        let name = get_package_name(tmp.path()).unwrap();
        assert_eq!(name, "correct-name");
    }

    // ==================== find_package_dir tests ====================

    #[test]
    fn find_package_dir_as_path() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("my-crate");
        fs::create_dir(&crate_dir).unwrap();
        fs::write(crate_dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();

        // Pass path directly
        let found = find_package_dir(tmp.path(), crate_dir.to_str().unwrap()).unwrap();
        assert_eq!(found.file_name().unwrap(), "my-crate");
    }

    #[test]
    fn find_package_dir_in_libs() {
        let tmp = TempDir::new().unwrap();
        let libs_dir = tmp.path().join("libs").join("my-lib");
        fs::create_dir_all(&libs_dir).unwrap();
        fs::write(libs_dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();

        let found = find_package_dir(tmp.path(), "my-lib").unwrap();
        assert_eq!(found, libs_dir);
    }

    #[test]
    fn find_package_dir_in_apps() {
        let tmp = TempDir::new().unwrap();
        let apps_dir = tmp.path().join("apps").join("my-app");
        fs::create_dir_all(&apps_dir).unwrap();
        fs::write(apps_dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();

        let found = find_package_dir(tmp.path(), "my-app").unwrap();
        assert_eq!(found, apps_dir);
    }

    #[test]
    fn find_package_dir_libs_preferred_over_apps() {
        let tmp = TempDir::new().unwrap();
        // Create both libs/foo and apps/foo
        let libs_foo = tmp.path().join("libs").join("foo");
        let apps_foo = tmp.path().join("apps").join("foo");
        fs::create_dir_all(&libs_foo).unwrap();
        fs::create_dir_all(&apps_foo).unwrap();
        fs::write(libs_foo.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::write(apps_foo.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();

        // libs/ is checked first
        let found = find_package_dir(tmp.path(), "foo").unwrap();
        assert_eq!(found, libs_foo);
    }

    #[test]
    fn find_package_dir_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = find_package_dir(tmp.path(), "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // ==================== find_tspec tests ====================

    #[test]
    fn find_tspec_explicit_path() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        let spec_dir = tmp.path().join("specs");
        fs::create_dir(&crate_dir).unwrap();
        fs::create_dir(&spec_dir).unwrap();

        let tspec_path = spec_dir.join("custom.toml");
        fs::write(&tspec_path, "# spec").unwrap();

        // Explicit path should be found
        let found = find_tspec(&crate_dir, Some(tspec_path.to_str().unwrap())).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().to_string_lossy().contains("custom.toml"));
    }

    #[test]
    fn find_tspec_explicit_relative_to_crate() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        let tspec_path = crate_dir.join("my-spec.toml");
        fs::write(&tspec_path, "# spec").unwrap();

        // Name without path, found in crate_dir
        let found = find_tspec(&crate_dir, Some("my-spec.toml")).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().to_string_lossy().contains("my-spec.toml"));
    }

    #[test]
    fn find_tspec_default_exists() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        let tspec_path = crate_dir.join(format!("tspec{}", SUFFIX));
        fs::write(&tspec_path, "# default tspec").unwrap();

        // No explicit name, should find default
        let found = find_tspec(&crate_dir, None).unwrap();
        assert!(found.is_some());
        assert!(
            found
                .unwrap()
                .to_string_lossy()
                .contains(&format!("tspec{}", SUFFIX))
        );
    }

    #[test]
    fn find_tspec_default_not_exists() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        // No default tspec, should return None (plain cargo build)
        let found = find_tspec(&crate_dir, None).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn find_tspec_explicit_not_found() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        let result = find_tspec(&crate_dir, Some("nonexistent.toml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn find_tspec_suffix_fallback() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        // Create optimized{SUFFIX}
        let tspec_path = crate_dir.join(format!("optimized{}", SUFFIX));
        fs::write(&tspec_path, "# optimized tspec").unwrap();

        // Request "optimized" without extension, should find optimized{SUFFIX}
        let found = find_tspec(&crate_dir, Some("optimized")).unwrap();
        assert!(found.is_some());
        assert!(
            found
                .unwrap()
                .to_string_lossy()
                .contains(&format!("optimized{}", SUFFIX))
        );
    }

    #[test]
    fn find_tspec_no_suffix_fallback_when_has_extension() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        // Create foo{SUFFIX} but request foo.toml (has extension)
        let tspec_path = crate_dir.join(format!("foo{}", SUFFIX));
        fs::write(&tspec_path, "# tspec").unwrap();

        // Request "foo.toml" - should NOT try suffix fallback
        let result = find_tspec(&crate_dir, Some("foo.toml"));
        assert!(result.is_err());
    }

    // ==================== find_tspecs tests ====================

    #[test]
    fn find_tspecs_default_pattern() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        let default_name = format!("tspec{}", SUFFIX);
        let opt_name = format!("tspec-opt{}", SUFFIX);
        fs::write(crate_dir.join(&default_name), "# default").unwrap();
        fs::write(crate_dir.join(&opt_name), "# opt").unwrap();
        fs::write(crate_dir.join("other.toml"), "# other").unwrap();

        // Empty patterns = default "tspec*{SUFFIX}"
        let found = find_tspecs(&crate_dir, &[]).unwrap();
        assert_eq!(found.len(), 2);
        assert!(
            found
                .iter()
                .any(|p| p.file_name().unwrap() == default_name.as_str())
        );
        assert!(
            found
                .iter()
                .any(|p| p.file_name().unwrap() == opt_name.as_str())
        );
    }

    #[test]
    fn find_tspecs_explicit_glob() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        let opt_name = format!("tspec-opt{}", SUFFIX);
        fs::write(crate_dir.join(format!("tspec{}", SUFFIX)), "# default").unwrap();
        fs::write(crate_dir.join(&opt_name), "# opt").unwrap();
        fs::write(crate_dir.join(format!("other{}", SUFFIX)), "# other").unwrap();

        let found = find_tspecs(&crate_dir, &["*-opt*".to_string()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), opt_name.as_str());
    }

    #[test]
    fn find_tspecs_explicit_files() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        fs::write(crate_dir.join("a.toml"), "# a").unwrap();
        fs::write(crate_dir.join("b.toml"), "# b").unwrap();
        fs::write(crate_dir.join("c.toml"), "# c").unwrap();

        let found = find_tspecs(&crate_dir, &["a.toml".to_string(), "c.toml".to_string()]).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|p| p.file_name().unwrap() == "a.toml"));
        assert!(found.iter().any(|p| p.file_name().unwrap() == "c.toml"));
    }

    #[test]
    fn find_tspecs_sorted_and_deduped() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        fs::write(crate_dir.join("z.toml"), "# z").unwrap();
        fs::write(crate_dir.join("a.toml"), "# a").unwrap();

        // Request same file twice via different patterns
        let found = find_tspecs(&crate_dir, &["*.toml".to_string(), "a.toml".to_string()]).unwrap();
        assert_eq!(found.len(), 2); // Deduped
        assert_eq!(found[0].file_name().unwrap(), "a.toml"); // Sorted
        assert_eq!(found[1].file_name().unwrap(), "z.toml");
    }

    #[test]
    fn find_tspecs_none_found_errors() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        let result = find_tspecs(&crate_dir, &[]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("no tspec files found")
        );
    }

    #[test]
    fn find_tspecs_glob_matches_multi_dot_filenames() {
        let tmp = TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crate");
        fs::create_dir(&crate_dir).unwrap();

        let default_name = format!("tspec{}", SUFFIX);
        let musl_name = format!("tspec.musl{}", SUFFIX);
        let other_name = format!("t1{}", SUFFIX);
        fs::write(crate_dir.join(&default_name), "# default").unwrap();
        fs::write(crate_dir.join(&musl_name), "# musl").unwrap();
        fs::write(crate_dir.join(&other_name), "# other").unwrap();

        // Default pattern should match tspec* but not t1*
        let found = find_tspecs(&crate_dir, &[]).unwrap();
        assert_eq!(found.len(), 2);
        assert!(
            found
                .iter()
                .any(|p| p.file_name().unwrap() == default_name.as_str())
        );
        assert!(
            found
                .iter()
                .any(|p| p.file_name().unwrap() == musl_name.as_str())
        );

        // Wildcard should match all three
        let found_all = find_tspecs(&crate_dir, &[format!("*{}", SUFFIX)]).unwrap();
        assert_eq!(found_all.len(), 3);
    }

    // ==================== get_binary_path tests ====================

    #[test]
    fn get_binary_path_simple_debug() {
        let workspace = Path::new("/workspace");
        let path = get_binary_path_simple(workspace, "myapp", None);
        assert_eq!(path, PathBuf::from("/workspace/target/debug/myapp"));
    }

    #[test]
    fn get_binary_path_simple_release() {
        let workspace = Path::new("/workspace");
        let path = get_binary_path_simple(workspace, "myapp", Some("release"));
        assert_eq!(path, PathBuf::from("/workspace/target/release/myapp"));
    }

    #[test]
    fn get_binary_path_simple_custom_profile() {
        let workspace = Path::new("/workspace");
        let path = get_binary_path_simple(workspace, "myapp", Some("release-small"));
        assert_eq!(path, PathBuf::from("/workspace/target/release-small/myapp"));
    }

    #[test]
    fn get_binary_path_empty_spec_debug() {
        let workspace = Path::new("/workspace");
        let spec = Spec::default();
        let path = get_binary_path(workspace, "myapp", &spec, None, None);
        assert_eq!(path, PathBuf::from("/workspace/target/debug/myapp"));
    }

    #[test]
    fn get_binary_path_empty_spec_release_flag() {
        let workspace = Path::new("/workspace");
        let spec = Spec::default();
        let path = get_binary_path(workspace, "myapp", &spec, Some("release"), None);
        assert_eq!(path, PathBuf::from("/workspace/target/release/myapp"));
    }

    #[test]
    fn get_binary_path_release_from_spec_profile() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                profile: Some("release".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        // cli_profile=None but spec says release
        let path = get_binary_path(workspace, "myapp", &spec, None, None);
        assert_eq!(path, PathBuf::from("/workspace/target/release/myapp"));
    }

    #[test]
    fn get_binary_path_spec_profile_overrides_cli() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                profile: Some("release-small".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        // cli_profile says release, but spec says release-small — spec wins
        let path = get_binary_path(workspace, "myapp", &spec, Some("release"), None);
        assert_eq!(path, PathBuf::from("/workspace/target/release-small/myapp"));
    }

    #[test]
    fn get_binary_path_with_target_triple() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                target_triple: Some("x86_64-unknown-linux-musl".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let path = get_binary_path(workspace, "myapp", &spec, Some("release"), None);
        assert_eq!(
            path,
            PathBuf::from("/workspace/target/x86_64-unknown-linux-musl/release/myapp")
        );
    }

    #[test]
    fn get_binary_path_with_target_json() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                target_json: Some(PathBuf::from("x86_64-unknown-linux-rlibcx2.json")),
                ..Default::default()
            },
            ..Default::default()
        };
        let path = get_binary_path(workspace, "myapp", &spec, Some("release"), None);
        assert_eq!(
            path,
            PathBuf::from("/workspace/target/x86_64-unknown-linux-rlibcx2/release/myapp")
        );
    }

    #[test]
    fn get_binary_path_target_triple_debug() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                target_triple: Some("x86_64-unknown-linux-musl".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let path = get_binary_path(workspace, "myapp", &spec, None, None);
        assert_eq!(
            path,
            PathBuf::from("/workspace/target/x86_64-unknown-linux-musl/debug/myapp")
        );
    }

    #[test]
    fn get_binary_path_debug_from_spec_profile() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                profile: Some("debug".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let path = get_binary_path(workspace, "myapp", &spec, None, None);
        assert_eq!(path, PathBuf::from("/workspace/target/debug/myapp"));
    }

    #[test]
    fn get_binary_path_custom_profile_from_spec() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                profile: Some("release-small".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let path = get_binary_path(workspace, "myapp", &spec, None, None);
        assert_eq!(path, PathBuf::from("/workspace/target/release-small/myapp"));
    }

    #[test]
    fn get_binary_path_dev_profile_maps_to_debug() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                profile: Some("dev".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let path = get_binary_path(workspace, "myapp", &spec, None, None);
        assert_eq!(path, PathBuf::from("/workspace/target/debug/myapp"));
    }

    // ==================== get_binary_path with target_dir tests ====================

    #[test]
    fn get_binary_path_with_target_dir_and_triple() {
        use crate::types::CargoConfig;
        let workspace = Path::new("/workspace");
        let spec = Spec {
            cargo: CargoConfig {
                target_triple: Some("x86_64-unknown-linux-musl".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let path = get_binary_path(
            workspace,
            "myapp",
            &spec,
            Some("release"),
            Some("static-opt"),
        );
        assert_eq!(
            path,
            PathBuf::from("/workspace/target/static-opt/x86_64-unknown-linux-musl/release/myapp")
        );
    }

    #[test]
    fn get_binary_path_with_target_dir_no_triple() {
        let workspace = Path::new("/workspace");
        let spec = Spec::default();
        let path = get_binary_path(workspace, "myapp", &spec, None, Some("custom"));
        assert_eq!(path, PathBuf::from("/workspace/target/custom/debug/myapp"));
    }

    // ==================== is_excluded_from_workspace tests ====================

    #[test]
    fn excluded_exact_match() {
        let ws_content = r#"
[workspace]
members = ["crates/foo"]
exclude = ["tests/fixtures"]
"#;
        let ws_dir = Path::new("/project");
        let pkg_dir = Path::new("/project/tests/fixtures");
        assert!(is_excluded_from_workspace(ws_content, ws_dir, pkg_dir));
    }

    #[test]
    fn excluded_nested_match() {
        let ws_content = r#"
[workspace]
members = ["crates/foo"]
exclude = ["tests/fixtures"]
"#;
        let ws_dir = Path::new("/project");
        let pkg_dir = Path::new("/project/tests/fixtures/pop");
        assert!(is_excluded_from_workspace(ws_content, ws_dir, pkg_dir));
    }

    #[test]
    fn not_excluded_member() {
        let ws_content = r#"
[workspace]
members = ["crates/foo"]
exclude = ["tests/fixtures"]
"#;
        let ws_dir = Path::new("/project");
        let pkg_dir = Path::new("/project/crates/foo");
        assert!(!is_excluded_from_workspace(ws_content, ws_dir, pkg_dir));
    }

    #[test]
    fn not_excluded_no_exclude_list() {
        let ws_content = r#"
[workspace]
members = ["crates/foo"]
"#;
        let ws_dir = Path::new("/project");
        let pkg_dir = Path::new("/project/crates/foo");
        assert!(!is_excluded_from_workspace(ws_content, ws_dir, pkg_dir));
    }

    // ==================== resolve_manifest_path tests ====================

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn resolve_mp_workspace_dir() {
        let root = resolve_manifest_path(&fixture_path("popws-3p")).unwrap();
        assert_eq!(root, fixture_path("popws-3p").canonicalize().unwrap());
    }

    #[test]
    fn resolve_mp_subpackage_dir() {
        let path = fixture_path("popws-3p/app-a");
        let root = resolve_manifest_path(&path).unwrap();
        // Should resolve to workspace root, not the sub-package
        assert_eq!(root, fixture_path("popws-3p").canonicalize().unwrap());
    }

    #[test]
    fn resolve_mp_subpackage_cargo_toml() {
        let path = fixture_path("popws-3p/lib-b/Cargo.toml");
        let root = resolve_manifest_path(&path).unwrap();
        assert_eq!(root, fixture_path("popws-3p").canonicalize().unwrap());
    }

    #[test]
    fn resolve_mp_pop_fixture() {
        let root = resolve_manifest_path(&fixture_path("pop")).unwrap();
        assert_eq!(root, fixture_path("pop").canonicalize().unwrap());
    }

    #[test]
    fn resolve_mp_nonexistent_path() {
        let result = resolve_manifest_path(Path::new("/no/such/path"));
        assert!(result.is_err());
    }
}
