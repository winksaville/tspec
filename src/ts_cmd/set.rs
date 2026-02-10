//! `tspec ts set` - Set a value in a tspec using toml_edit (preserves comments/formatting)

use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::DocumentMut;

use super::edit::{self, SetOp};
use crate::find_paths::{find_tspec, resolve_package_dir};

/// Set/append/remove a value in a tspec and save in place
pub fn set_value(
    project_root: &Path,
    package: Option<&str>,
    key: &str,
    value: &str,
    op: SetOp,
    tspec: Option<&str>,
) -> Result<()> {
    let workspace = project_root;
    let package_dir = resolve_package_dir(workspace, package)?;

    // Resolve tspec path (existing or new)
    let output_path = match find_tspec(&package_dir, tspec)? {
        Some(path) => path,
        None => {
            let base_name = match tspec {
                Some(t) => t
                    .strip_suffix(crate::TSPEC_SUFFIX)
                    .or_else(|| t.strip_suffix(".toml"))
                    .unwrap_or(t),
                None => "tspec",
            };
            package_dir.join(format!("{}{}", base_name, crate::TSPEC_SUFFIX))
        }
    };

    // Validate key and value
    let kind = edit::validate_key(key)?;

    // Append/remove only make sense for array fields
    if op != SetOp::Replace && kind != edit::FieldKind::Array {
        anyhow::bail!(
            "operator += and -= can only be used with array fields, but '{}' is a scalar",
            key
        );
    }

    // Only validate enum constraints for replace (append/remove are raw strings)
    if op == SetOp::Replace {
        edit::validate_value(key, value)?;
    }

    // Read existing content or start empty
    let content = if output_path.exists() {
        std::fs::read_to_string(&output_path)
            .with_context(|| format!("failed to read: {}", output_path.display()))?
    } else {
        String::new()
    };

    // Parse, edit, write
    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("failed to parse: {}", output_path.display()))?;

    match op {
        SetOp::Replace => edit::set_field(&mut doc, key, value, kind)?,
        SetOp::Append => edit::append_field(&mut doc, key, value)?,
        SetOp::Remove => edit::remove_from_field(&mut doc, key, value)?,
    }

    std::fs::write(&output_path, doc.to_string())
        .with_context(|| format!("failed to write: {}", output_path.display()))?;

    println!(
        "Saved {}",
        output_path
            .strip_prefix(workspace)
            .unwrap_or(&output_path)
            .display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_constants::SUFFIX;
    use crate::tspec::load_spec;
    use tempfile::TempDir;

    /// Helper: create a tspec file with given content and run set on it.
    /// Returns (TempDir, path, output_string) - TempDir must stay alive for path to be valid.
    fn set_in_file(content: &str, key: &str, value: &str) -> (TempDir, std::path::PathBuf, String) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("tspec{}", SUFFIX));
        std::fs::write(&path, content).unwrap();

        let mut doc: DocumentMut = content.parse().unwrap();
        let kind = edit::validate_key(key).unwrap();
        edit::validate_value(key, value).unwrap();
        edit::set_field(&mut doc, key, value, kind).unwrap();
        let output = doc.to_string();
        std::fs::write(&path, &output).unwrap();

        (dir, path, output)
    }

    #[test]
    fn set_strip_mode() {
        let (_dir, path, _) = set_in_file("", "strip", "symbols");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.strip, Some(crate::options::StripMode::Symbols));
    }

    #[test]
    fn set_panic_mode() {
        let (_dir, path, _) = set_in_file("", "panic", "abort");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.panic, Some(crate::options::PanicMode::Abort));
    }

    #[test]
    fn set_rustc_lto() {
        let (_dir, path, _) = set_in_file("", "rustc.lto", "true");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.rustc.lto, Some(true));
    }

    #[test]
    fn set_rustc_opt_level() {
        let (_dir, path, _) = set_in_file("", "rustc.opt_level", "z");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.rustc.opt_level, Some(crate::types::OptLevel::Oz));
    }

    #[test]
    fn set_cargo_profile() {
        let (_dir, path, _) = set_in_file("", "cargo.profile", "release");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.cargo.profile, Some(crate::types::Profile::Release));
    }

    #[test]
    fn unknown_key_errors() {
        let result = edit::validate_key("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown key"));
    }

    #[test]
    fn invalid_strip_mode_errors() {
        let result = edit::validate_value("strip", "invalid");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid strip mode")
        );
    }

    #[test]
    fn set_cargo_target_dir() {
        let (_dir, path, _) = set_in_file("", "cargo.target_dir", "<name>");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.cargo.target_dir, Some("<name>".to_string()));
    }

    #[test]
    fn set_cargo_target_triple() {
        let (_dir, path, _) = set_in_file("", "cargo.target_triple", "my custom triple");
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.cargo.target_triple,
            Some("my custom triple".to_string())
        );
    }

    #[test]
    fn set_rustc_build_std() {
        let (_dir, path, _) = set_in_file("", "rustc.build_std", r#"["core", "alloc"]"#);
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.rustc.build_std,
            vec!["core".to_string(), "alloc".to_string()]
        );
    }

    #[test]
    fn set_linker_args() {
        let (_dir, path, _) = set_in_file("", "linker.args", r#"["-static", "-nostdlib"]"#);
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.linker.args,
            vec!["-static".to_string(), "-nostdlib".to_string()]
        );
    }

    #[test]
    fn set_cargo_unstable() {
        let (_dir, path, _) = set_in_file("", "cargo.unstable", r#"["panic-immediate-abort"]"#);
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.cargo.unstable,
            vec!["panic-immediate-abort".to_string()]
        );
    }

    #[test]
    fn set_rustc_flags() {
        let (_dir, path, _) = set_in_file("", "rustc.flags", r#"["-Cforce-frame-pointers=yes"]"#);
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.rustc.flags,
            vec!["-Cforce-frame-pointers=yes".to_string()]
        );
    }

    #[test]
    fn set_preserves_comments() {
        let input = "# Important comment\npanic = \"unwind\"\n";
        let (_dir, _, output) = set_in_file(input, "strip", "symbols");
        assert!(output.contains("# Important comment"));
    }

    #[test]
    fn set_codegen_units() {
        let (_dir, path, _) = set_in_file("", "rustc.codegen_units", "1");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.rustc.codegen_units, Some(1));
    }

    /// Helper for append/remove operations on a tspec file.
    fn op_in_file(
        content: &str,
        key: &str,
        value: &str,
        op: SetOp,
    ) -> (TempDir, std::path::PathBuf, String) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("tspec{}", SUFFIX));
        std::fs::write(&path, content).unwrap();

        let mut doc: DocumentMut = content.parse().unwrap();
        match op {
            SetOp::Append => edit::append_field(&mut doc, key, value).unwrap(),
            SetOp::Remove => edit::remove_from_field(&mut doc, key, value).unwrap(),
            SetOp::Replace => {
                let kind = edit::validate_key(key).unwrap();
                edit::set_field(&mut doc, key, value, kind).unwrap();
            }
        }
        let output = doc.to_string();
        std::fs::write(&path, &output).unwrap();

        (dir, path, output)
    }

    #[test]
    fn append_to_empty_array() {
        let (_dir, path, _) = op_in_file("", "linker.args", "-static", SetOp::Append);
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-static".to_string()]);
    }

    #[test]
    fn append_to_existing_array() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = op_in_file(input, "linker.args", "-Wl,--gc-sections", SetOp::Append);
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.linker.args,
            vec!["-static".to_string(), "-Wl,--gc-sections".to_string()]
        );
    }

    #[test]
    fn append_multiple_values_bracket_syntax() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = op_in_file(
            input,
            "linker.args",
            r#"["-nostdlib", "-Wl,--gc-sections"]"#,
            SetOp::Append,
        );
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.linker.args,
            vec![
                "-static".to_string(),
                "-nostdlib".to_string(),
                "-Wl,--gc-sections".to_string()
            ]
        );
    }

    #[test]
    fn append_skips_duplicates() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = op_in_file(input, "linker.args", "-static", SetOp::Append);
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-static".to_string()]);
    }

    #[test]
    fn remove_from_array() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let (_dir, path, _) = op_in_file(input, "linker.args", "-nostdlib", SetOp::Remove);
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-static".to_string()]);
    }

    #[test]
    fn remove_multiple_from_array_bracket_syntax() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\", \"-Wl,--gc-sections\"]\n";
        let (_dir, path, _) = op_in_file(
            input,
            "linker.args",
            r#"["-static", "-Wl,--gc-sections"]"#,
            SetOp::Remove,
        );
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-nostdlib".to_string()]);
    }

    #[test]
    fn remove_last_entry_removes_field() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = op_in_file(input, "linker.args", "-static", SetOp::Remove);
        let spec = load_spec(&path).unwrap();
        assert!(spec.linker.args.is_empty());
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = op_in_file(input, "linker.args", "-nostdlib", SetOp::Remove);
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-static".to_string()]);
    }
}
