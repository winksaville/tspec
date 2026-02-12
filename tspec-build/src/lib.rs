use serde::Deserialize;
use std::path::Path;

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

/// Emit `cargo:rustc-link-arg-bin=` directives from the tspec's linker.args.
///
/// Reads the spec file path from the `TSPEC_SPEC_FILE` environment variable
/// (set by tspec before invoking cargo) and the binary name from `CARGO_PKG_NAME`
/// (set by cargo in build scripts).
///
/// Call this from your `build.rs`:
/// ```no_run
/// // your existing build logic here
/// tspec_build::emit_linker_flags();
/// ```
///
/// If `TSPEC_SPEC_FILE` is not set (e.g., building with plain cargo), this is a no-op.
pub fn emit_linker_flags() {
    let spec_file = match std::env::var("TSPEC_SPEC_FILE") {
        Ok(path) => path,
        Err(_) => return, // Not building via tspec, nothing to do
    };

    let bin_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME not set");

    let args = read_linker_args(Path::new(&spec_file));

    for arg in &args {
        println!("cargo:rustc-link-arg-bin={bin_name}={arg}");
    }

    // Rebuild if the spec file changes
    println!("cargo:rerun-if-changed={spec_file}");
    println!("cargo:rerun-if-env-changed=TSPEC_SPEC_FILE");
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
}
