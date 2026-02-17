//! `tspec ts set` - Set a value in a tspec using toml_edit (preserves comments/formatting)

use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::DocumentMut;

use super::edit::{self, FieldKind};
use crate::find_paths::{find_tspec, resolve_package_dir};

/// Set a field in a tspec (scalar or replace entire array) and save in place
pub fn set_value(
    project_root: &Path,
    package: Option<&str>,
    key: &str,
    values: &[String],
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

    // Handle Table fields with sub-keys
    if kind == FieldKind::Table {
        if let Some((table_path, sub_key)) = edit::parse_table_key(key) {
            if values.len() != 1 {
                anyhow::bail!(
                    "table sub-key '{}' requires exactly one value, got {}",
                    key,
                    values.len()
                );
            }

            let content = if output_path.exists() {
                std::fs::read_to_string(&output_path)
                    .with_context(|| format!("failed to read: {}", output_path.display()))?
            } else {
                String::new()
            };

            let mut doc: DocumentMut = content
                .parse()
                .with_context(|| format!("failed to parse: {}", output_path.display()))?;

            edit::set_table_value(&mut doc, table_path, sub_key, &values[0])?;

            std::fs::write(&output_path, doc.to_string())
                .with_context(|| format!("failed to write: {}", output_path.display()))?;

            println!(
                "Saved {}",
                output_path
                    .strip_prefix(workspace)
                    .unwrap_or(&output_path)
                    .display()
            );
            return Ok(());
        } else {
            anyhow::bail!(
                "'ts set' on table field '{}' requires a sub-key, e.g. {}.\"key\"",
                key,
                key
            );
        }
    }

    // Validate enum constraints for scalar fields
    if kind == FieldKind::Scalar {
        if values.len() != 1 {
            anyhow::bail!(
                "scalar field '{}' requires exactly one value, got {}",
                key,
                values.len()
            );
        }
        edit::validate_value(key, &values[0])?;
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

    edit::set_field(&mut doc, key, values, kind)?;

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
    use crate::test_constants::SUFFIX;
    use crate::tspec::load_spec;
    use tempfile::TempDir;
    use toml_edit::DocumentMut;

    use super::super::edit;

    fn vs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// Helper: create a tspec file with given content and run set on it.
    fn set_in_file(
        content: &str,
        key: &str,
        values: &[String],
    ) -> (TempDir, std::path::PathBuf, String) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("tspec{}", SUFFIX));
        std::fs::write(&path, content).unwrap();

        let mut doc: DocumentMut = content.parse().unwrap();
        let kind = edit::validate_key(key).unwrap();
        if kind == edit::FieldKind::Scalar {
            edit::validate_value(key, &values[0]).unwrap();
        }
        edit::set_field(&mut doc, key, values, kind).unwrap();
        let output = doc.to_string();
        std::fs::write(&path, &output).unwrap();

        (dir, path, output)
    }

    #[test]
    fn set_strip_mode() {
        let (_dir, path, _) = set_in_file("", "strip", &vs(&["symbols"]));
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.strip, Some(crate::options::StripMode::Symbols));
    }

    #[test]
    fn set_panic_mode() {
        let (_dir, path, _) = set_in_file("", "panic", &vs(&["abort"]));
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.panic, Some(crate::options::PanicMode::Abort));
    }

    #[test]
    fn set_cargo_profile() {
        let (_dir, path, _) = set_in_file("", "cargo.profile", &vs(&["release"]));
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
        let (_dir, path, _) = set_in_file("", "cargo.target_dir", &vs(&["{name}"]));
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.cargo.target_dir, Some("{name}".to_string()));
    }

    #[test]
    fn set_cargo_target_triple() {
        let (_dir, path, _) = set_in_file("", "cargo.target_triple", &vs(&["my custom triple"]));
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.cargo.target_triple,
            Some("my custom triple".to_string())
        );
    }

    #[test]
    fn set_cargo_build_std() {
        let (_dir, path, _) = set_in_file("", "cargo.build_std", &vs(&["core", "alloc"]));
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.cargo.build_std,
            vec!["core".to_string(), "alloc".to_string()]
        );
    }

    #[test]
    fn set_linker_args() {
        let (_dir, path, _) = set_in_file("", "linker.args", &vs(&["-static", "-nostdlib"]));
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.linker.args,
            vec!["-static".to_string(), "-nostdlib".to_string()]
        );
    }

    #[test]
    fn set_cargo_unstable() {
        let (_dir, path, _) = set_in_file("", "cargo.unstable", &vs(&["panic-immediate-abort"]));
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.cargo.unstable,
            vec!["panic-immediate-abort".to_string()]
        );
    }

    #[test]
    fn set_rustflags() {
        let (_dir, path, _) = set_in_file("", "rustflags", &vs(&["-Cforce-frame-pointers=yes"]));
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.rustflags,
            vec!["-Cforce-frame-pointers=yes".to_string()]
        );
    }

    #[test]
    fn set_preserves_comments() {
        let input = "# Important comment\npanic = \"unwind\"\n";
        let (_dir, _, output) = set_in_file(input, "strip", &vs(&["symbols"]));
        assert!(output.contains("# Important comment"));
    }

    // --- Table field (config_key_value) tests ---

    /// Helper for table sub-key set operations
    fn set_table_in_file(
        content: &str,
        key: &str,
        values: &[String],
    ) -> (tempfile::TempDir, std::path::PathBuf, String) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(format!("tspec{}", SUFFIX));
        std::fs::write(&path, content).unwrap();

        let kind = edit::validate_key(key).unwrap();
        assert_eq!(kind, edit::FieldKind::Table);
        let (table_path, sub_key) = edit::parse_table_key(key).unwrap();

        let mut doc: DocumentMut = content.parse().unwrap();
        edit::set_table_value(&mut doc, table_path, sub_key, &values[0]).unwrap();
        let output = doc.to_string();
        std::fs::write(&path, &output).unwrap();

        (dir, path, output)
    }

    #[test]
    fn set_config_kv_string() {
        let (_dir, path, _) = set_table_in_file(
            "",
            "cargo.config_key_value.\"profile.release.opt-level\"",
            &vs(&["s"]),
        );
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.cargo.config_key_value.get("profile.release.opt-level"),
            Some(&crate::types::ConfigValue::String("s".to_string()))
        );
    }

    #[test]
    fn set_config_kv_bool() {
        let (_dir, path, _) = set_table_in_file(
            "",
            "cargo.config_key_value.\"profile.release.lto\"",
            &vs(&["true"]),
        );
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.cargo.config_key_value.get("profile.release.lto"),
            Some(&crate::types::ConfigValue::Bool(true))
        );
    }

    #[test]
    fn set_config_kv_integer() {
        let (_dir, path, _) = set_table_in_file(
            "",
            "cargo.config_key_value.\"profile.release.codegen-units\"",
            &vs(&["1"]),
        );
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.cargo
                .config_key_value
                .get("profile.release.codegen-units"),
            Some(&crate::types::ConfigValue::Integer(1))
        );
    }

    #[test]
    fn set_config_kv_bare_table_errors() {
        let result = edit::parse_table_key("cargo.config_key_value");
        assert!(result.is_none());
    }
}
