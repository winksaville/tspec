//! `tspec ts remove` - Remove items from an array field in a tspec (by value or index)

use anyhow::{Context, Result, bail};
use std::path::Path;
use toml_edit::DocumentMut;

use super::edit::{self, FieldKind};
use crate::find_paths::{find_tspec, resolve_package_dir};

/// Remove items from an array field in a tspec (by value or by index)
pub fn remove_value(
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
            "'ts remove' only works on array fields, but '{}' is a scalar",
            key
        );
    }

    // Validate: either index or values, not both, not neither
    if index.is_some() && !values.is_empty() {
        bail!(
            "cannot use both --index and values; use --index to remove by position, or provide values to remove by value"
        );
    }
    if index.is_none() && values.is_empty() {
        bail!("provide values to remove, or use --index to remove by position");
    }

    // Read, parse, edit, write
    let content = std::fs::read_to_string(&output_path)
        .with_context(|| format!("failed to read: {}", output_path.display()))?;

    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("failed to parse: {}", output_path.display()))?;

    if let Some(idx) = index {
        edit::remove_item_by_index(&mut doc, key, idx)?;
    } else {
        edit::remove_items_by_value(&mut doc, key, values)?;
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
    use crate::test_constants::SUFFIX;
    use crate::tspec::load_spec;
    use tempfile::TempDir;
    use toml_edit::DocumentMut;

    use super::super::edit;

    fn vs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn remove_in_file(
        content: &str,
        key: &str,
        values: &[String],
        index: Option<usize>,
    ) -> (TempDir, std::path::PathBuf, String) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("tspec{}", SUFFIX));
        std::fs::write(&path, content).unwrap();

        let mut doc: DocumentMut = content.parse().unwrap();
        if let Some(idx) = index {
            edit::remove_item_by_index(&mut doc, key, idx).unwrap();
        } else {
            edit::remove_items_by_value(&mut doc, key, values).unwrap();
        }
        let output = doc.to_string();
        std::fs::write(&path, &output).unwrap();

        (dir, path, output)
    }

    #[test]
    fn remove_by_value() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let (_dir, path, _) = remove_in_file(input, "linker.args", &vs(&["-nostdlib"]), None);
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-static".to_string()]);
    }

    #[test]
    fn remove_multiple_by_value() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\", \"-Wl,--gc-sections\"]\n";
        let (_dir, path, _) = remove_in_file(
            input,
            "linker.args",
            &vs(&["-static", "-Wl,--gc-sections"]),
            None,
        );
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-nostdlib".to_string()]);
    }

    #[test]
    fn remove_last_keeps_empty_array() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = remove_in_file(input, "linker.args", &vs(&["-static"]), None);
        let spec = load_spec(&path).unwrap();
        assert!(spec.linker.args.is_empty());
    }

    #[test]
    fn remove_by_index() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let (_dir, path, _) = remove_in_file(input, "linker.args", &[], Some(0));
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-nostdlib".to_string()]);
    }

    #[test]
    fn remove_by_index_last_keeps_empty_array() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = remove_in_file(input, "linker.args", &[], Some(0));
        let spec = load_spec(&path).unwrap();
        assert!(spec.linker.args.is_empty());
    }

    #[test]
    fn remove_nonexistent_value_is_noop() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let (_dir, path, _) = remove_in_file(input, "linker.args", &vs(&["-nostdlib"]), None);
        let spec = load_spec(&path).unwrap();
        assert_eq!(spec.linker.args, vec!["-static".to_string()]);
    }
}
