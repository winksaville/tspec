use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Minimal spec representation — only what we need for build.rs
#[derive(Deserialize, Default)]
struct Spec {
    #[serde(default)]
    linker: LinkerConfig,
}

#[derive(Deserialize, Default)]
struct LinkerConfig {
    #[serde(default)]
    args: Vec<String>,
}

/// Emit `cargo:rustc-link-arg-bin=` directives from linker.args in a tspec spec.
///
/// With a path, reads the spec file directly (relative to `CARGO_MANIFEST_DIR`).
/// This makes `cargo build` work without tspec for packages that need linker args:
/// ```no_run
/// tspec_build::emit_linker_flags_from(Some("tspec.ts.toml"));
/// ```
///
/// With `None`, reads the spec file path from the `TSPEC_SPEC_FILE` environment
/// variable (set by tspec before invoking cargo). This is a no-op if the variable
/// is not set, allowing the build.rs to work with both tspec and plain cargo:
/// ```no_run
/// tspec_build::emit_linker_flags_from(None);
/// ```
pub fn emit_linker_flags_from(spec_path: Option<&str>) {
    let (path, from_env) = match resolve_spec_path(spec_path) {
        Some(result) => result,
        None => return, // No spec_path and no env var — nothing to do
    };

    let bin_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME not set");

    let args = read_linker_args(&path);

    for arg in &args {
        println!("cargo:rustc-link-arg-bin={bin_name}={arg}");
    }

    // Rebuild if the spec file changes
    println!("cargo:rerun-if-changed={}", path.display());
    if from_env {
        println!("cargo:rerun-if-env-changed=TSPEC_SPEC_FILE");
    }
}

/// Resolve the spec file path. Returns (path, from_env) or None if no path available.
fn resolve_spec_path(spec_path: Option<&str>) -> Option<(PathBuf, bool)> {
    match spec_path {
        Some(p) => {
            let manifest_dir =
                std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
            Some((Path::new(&manifest_dir).join(p), false))
        }
        None => {
            let path = std::env::var("TSPEC_SPEC_FILE").ok()?;
            Some((PathBuf::from(path), true))
        }
    }
}

/// Read linker.args from a spec file. Returns empty vec on any error.
fn read_linker_args(path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let spec: Spec = match toml::from_str(&content) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    spec.linker.args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn read_linker_args_from_spec() {
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("test.ts.toml");
        let mut f = std::fs::File::create(&spec_path).unwrap();
        writeln!(
            f,
            r#"[linker]
args = ["-static", "-nostdlib"]
"#
        )
        .unwrap();

        let args = read_linker_args(&spec_path);
        assert_eq!(args, vec!["-static", "-nostdlib"]);
    }

    #[test]
    fn read_linker_args_empty_spec() {
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("empty.ts.toml");
        std::fs::write(&spec_path, "").unwrap();

        let args = read_linker_args(&spec_path);
        assert!(args.is_empty());
    }

    #[test]
    fn read_linker_args_no_linker_section() {
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("no-linker.ts.toml");
        std::fs::write(
            &spec_path,
            r#"[cargo]
profile = "release"
"#,
        )
        .unwrap();

        let args = read_linker_args(&spec_path);
        assert!(args.is_empty());
    }

    #[test]
    fn read_linker_args_missing_file() {
        let args = read_linker_args(Path::new("/nonexistent/path.ts.toml"));
        assert!(args.is_empty());
    }

    #[test]
    fn resolve_spec_path_with_explicit_path() {
        // CARGO_MANIFEST_DIR is set by cargo during tests
        let result = resolve_spec_path(Some("tspec.ts.toml"));
        assert!(result.is_some());
        let (path, from_env) = result.unwrap();
        assert!(path.ends_with("tspec.ts.toml"));
        assert!(!from_env);
    }

    #[test]
    fn resolve_spec_path_none_without_env() {
        // Ensure TSPEC_SPEC_FILE is not set
        unsafe { std::env::remove_var("TSPEC_SPEC_FILE") };
        let result = resolve_spec_path(None);
        assert!(result.is_none());
    }

    #[test]
    fn resolve_spec_path_none_with_env() {
        unsafe { std::env::set_var("TSPEC_SPEC_FILE", "/tmp/test.ts.toml") };
        let result = resolve_spec_path(None);
        unsafe { std::env::remove_var("TSPEC_SPEC_FILE") };

        assert!(result.is_some());
        let (path, from_env) = result.unwrap();
        assert_eq!(path, PathBuf::from("/tmp/test.ts.toml"));
        assert!(from_env);
    }

    #[test]
    fn emit_from_reads_spec_file() {
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("tspec.ts.toml");
        std::fs::write(
            &spec_path,
            r#"[linker]
args = ["-nostartfiles", "-static"]
"#,
        )
        .unwrap();

        // resolve_spec_path with explicit path uses CARGO_MANIFEST_DIR
        let (path, from_env) = resolve_spec_path(Some("tspec.ts.toml")).unwrap();
        assert!(!from_env);
        // Path is relative to CARGO_MANIFEST_DIR, not our temp dir,
        // so just verify the resolution logic works
        assert!(path.ends_with("tspec.ts.toml"));

        // Verify read_linker_args works with the temp file directly
        let args = read_linker_args(&spec_path);
        assert_eq!(args, vec!["-nostartfiles", "-static"]);
    }
}
