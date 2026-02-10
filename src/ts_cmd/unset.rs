//! `tspec ts unset` - Remove a field from a tspec (preserves comments/formatting)

use anyhow::{Context, Result, bail};
use std::path::Path;
use toml_edit::DocumentMut;

use super::edit;
use crate::find_paths::{find_tspec, resolve_package_dir};

/// Remove a field from a tspec
pub fn unset_value(
    project_root: &Path,
    package: Option<&str>,
    key: &str,
    tspec: Option<&str>,
) -> Result<()> {
    let workspace = project_root;
    let package_dir = resolve_package_dir(workspace, package)?;

    let output_path = match find_tspec(&package_dir, tspec)? {
        Some(path) => path,
        None => bail!("no tspec found to modify"),
    };

    // Validate the key
    edit::validate_key(key)?;

    // Read, parse, edit, write
    let content = std::fs::read_to_string(&output_path)
        .with_context(|| format!("failed to read: {}", output_path.display()))?;

    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("failed to parse: {}", output_path.display()))?;

    edit::unset_field(&mut doc, key)?;

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
    use toml_edit::DocumentMut;

    use super::*;

    fn unset_in_file(content: &str, key: &str) -> (tempfile::TempDir, std::path::PathBuf, String) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(format!("tspec{}", SUFFIX));
        std::fs::write(&path, content).unwrap();

        let mut doc: DocumentMut = content.parse().unwrap();
        edit::validate_key(key).unwrap();
        edit::unset_field(&mut doc, key).unwrap();
        let output = doc.to_string();
        std::fs::write(&path, &output).unwrap();

        (dir, path, output)
    }

    #[test]
    fn unset_toplevel_field() {
        let input = "panic = \"abort\"\nstrip = \"symbols\"\n";
        let (_dir, path, _) = unset_in_file(input, "panic");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.panic, None);
        assert_eq!(spec.strip, Some(crate::options::StripMode::Symbols));
    }

    #[test]
    fn unset_nested_field() {
        let input = "[rustc]\nlto = true\nopt_level = \"3\"\n";
        let (_dir, path, _) = unset_in_file(input, "rustc.lto");
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.rustc.lto, None);
        assert_eq!(spec.rustc.opt_level, Some(crate::types::OptLevel::O3));
    }

    #[test]
    fn unset_array_field() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = unset_in_file(input, "linker.args");
        let spec = load_spec(&path).unwrap();
        assert!(spec.linker.args.is_empty());
    }

    #[test]
    fn unset_preserves_comments() {
        // Comment is attached to strip (which stays), not to panic (which is removed)
        let input = "panic = \"abort\"\n# Keep this\nstrip = \"symbols\"\n";
        let (_dir, _, output) = unset_in_file(input, "panic");
        assert!(output.contains("# Keep this"));
        assert!(output.contains("strip = \"symbols\""));
    }

    #[test]
    fn unset_unknown_key_errors() {
        let result = edit::validate_key("nonexistent");
        assert!(result.is_err());
    }
}
