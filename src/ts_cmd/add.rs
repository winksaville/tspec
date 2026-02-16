//! `tspec ts add` - Add items to an array field in a tspec (preserves comments/formatting)

use anyhow::{Context, Result, bail};
use std::path::Path;
use toml_edit::DocumentMut;

use super::edit::{self, FieldKind};
use crate::find_paths::{find_tspec, resolve_package_dir};

/// Add items to an array field in a tspec
pub fn add_value(
    project_root: &Path,
    package: Option<&str>,
    key: &str,
    values: &[String],
    index: Option<usize>,
    tspec: Option<&str>,
) -> Result<()> {
    let workspace = project_root;
    let package_dir = resolve_package_dir(workspace, package)?;

    let output_path = match find_tspec(&package_dir, tspec)? {
        Some(path) => path,
        None => bail!("no tspec found to modify"),
    };

    // Validate key is an array field
    let kind = edit::validate_key(key)?;
    if kind != FieldKind::Array {
        bail!(
            "'ts add' only works on array fields, but '{}' is not an array field",
            key
        );
    }

    // Read, parse, edit, write
    let content = std::fs::read_to_string(&output_path)
        .with_context(|| format!("failed to read: {}", output_path.display()))?;

    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("failed to parse: {}", output_path.display()))?;

    edit::add_items(&mut doc, key, values, index)?;

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

    fn add_in_file(
        content: &str,
        key: &str,
        values: &[String],
        index: Option<usize>,
    ) -> (TempDir, std::path::PathBuf, String) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("tspec{}", SUFFIX));
        std::fs::write(&path, content).unwrap();

        let mut doc: DocumentMut = content.parse().unwrap();
        edit::add_items(&mut doc, key, values, index).unwrap();
        let output = doc.to_string();
        std::fs::write(&path, &output).unwrap();

        (dir, path, output)
    }

    #[test]
    fn append_single_to_empty() {
        let (_dir, path, _) = add_in_file("", "linker.args", &vs(&["-static"]), None);
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-static".to_string()]);
    }

    #[test]
    fn append_to_existing() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = add_in_file(input, "linker.args", &vs(&["-Wl,--gc-sections"]), None);
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.linker.args,
            vec!["-static".to_string(), "-Wl,--gc-sections".to_string()]
        );
    }

    #[test]
    fn append_deduplicates() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = add_in_file(input, "linker.args", &vs(&["-static"]), None);
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-static".to_string()]);
    }

    #[test]
    fn insert_at_beginning() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let (_dir, path, _) = add_in_file(input, "linker.args", &vs(&["-nostartfiles"]), Some(0));
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.linker.args,
            vec![
                "-nostartfiles".to_string(),
                "-static".to_string(),
                "-nostdlib".to_string()
            ]
        );
    }

    #[test]
    fn insert_at_middle() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let (_dir, path, _) =
            add_in_file(input, "linker.args", &vs(&["-Wl,--gc-sections"]), Some(1));
        let spec = load_spec(&path).unwrap();
        assert_eq!(
            spec.linker.args,
            vec![
                "-static".to_string(),
                "-Wl,--gc-sections".to_string(),
                "-nostdlib".to_string()
            ]
        );
    }

    #[test]
    fn scalar_key_rejected() {
        let kind = edit::validate_key("rustc.lto").unwrap();
        assert_eq!(kind, edit::FieldKind::Scalar);
    }
}
