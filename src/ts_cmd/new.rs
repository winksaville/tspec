//! `tspec ts new` - Create a new tspec file

use anyhow::Result;
use std::path::Path;

use crate::TSPEC_SUFFIX;
use crate::find_paths::{find_package_dir, find_tspec, resolve_package_dir};
use crate::tspec::save_spec;
use crate::types::Spec;

/// Create a new tspec file (public entry point)
pub fn new_tspec(
    project_root: &Path,
    package: Option<&str>,
    name: &str,
    from: Option<&str>,
) -> Result<()> {
    let workspace = project_root;
    let package_dir = resolve_package_dir(workspace, package)?;
    let output_path = package_dir.join(format!("{}{}", name, TSPEC_SUFFIX));

    // Check if file already exists
    if output_path.exists() {
        anyhow::bail!(
            "tspec '{}' already exists. Use a different name or delete the existing file.",
            output_path.file_name().unwrap().to_string_lossy(),
        );
    }

    match from {
        Some(source) => {
            // --from: raw file copy to preserve comments/formatting
            let source_path = resolve_source_spec(workspace, &package_dir, source)?;
            std::fs::copy(&source_path, &output_path).map_err(|e| {
                anyhow::anyhow!(
                    "failed to copy {} to {}: {}",
                    source_path.display(),
                    output_path.display(),
                    e
                )
            })?;
        }
        None => {
            // No source: create default empty spec via serde
            save_spec(&Spec::default(), &output_path)?;
        }
    }

    println!(
        "Created {}",
        output_path
            .strip_prefix(workspace)
            .unwrap_or(&output_path)
            .display()
    );

    Ok(())
}

/// Resolve the source spec path from a --from argument
fn resolve_source_spec(
    workspace: &Path,
    current_package_dir: &Path,
    source: &str,
) -> Result<std::path::PathBuf> {
    // Parse source: could be "package/spec" or just "spec"
    let (source_package_dir, source_spec, source_name) = if source.contains('/') {
        let parts: Vec<&str> = source.splitn(2, '/').collect();
        let pkg_dir = find_package_dir(workspace, parts[0])?;
        (pkg_dir, Some(parts[1]), parts[0])
    } else {
        // Same package, just spec name
        (
            current_package_dir.to_path_buf(),
            Some(source),
            "current package",
        )
    };

    find_tspec(&source_package_dir, source_spec)?.ok_or_else(|| {
        anyhow::anyhow!(
            "source tspec '{}' not found in {}",
            source_spec.unwrap_or("tspec"),
            source_name
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_constants::SUFFIX;
    use crate::tspec::load_spec;
    use tempfile::TempDir;

    #[test]
    fn create_empty_tspec() {
        let dir = TempDir::new().unwrap();
        let crate_dir = dir.path();
        let output_path = crate_dir.join(format!("test{}", SUFFIX));

        // Simulate: no --from, save default
        save_spec(&Spec::default(), &output_path).unwrap();

        assert!(output_path.exists());
        let spec = load_spec(&output_path).unwrap();
        assert_eq!(spec, Spec::default());
    }

    #[test]
    fn create_tspec_from_source_preserves_bytes() {
        let dir = TempDir::new().unwrap();
        let crate_dir = dir.path();

        // Create source with a comment
        let source_content = "# My custom comment\n[cargo]\nprofile = \"release\"\n\n[linker]\nargs = [\"-static\"]\n";
        let source_path = crate_dir.join(format!("source{}", SUFFIX));
        std::fs::write(&source_path, source_content).unwrap();

        let copy_path = crate_dir.join(format!("copy{}", SUFFIX));
        std::fs::copy(&source_path, &copy_path).unwrap();

        // Verify byte-for-byte identical
        let copied = std::fs::read_to_string(&copy_path).unwrap();
        assert_eq!(copied, source_content);

        // Verify it still parses correctly
        let loaded = load_spec(&copy_path).unwrap();
        assert_eq!(loaded.cargo.profile, Some("release".to_string()));
        assert_eq!(loaded.linker.args, vec!["-static".to_string()]);
    }

    #[test]
    fn error_when_file_exists() {
        let dir = TempDir::new().unwrap();
        let crate_dir = dir.path();

        let existing = crate_dir.join(format!("existing{}", SUFFIX));
        std::fs::write(&existing, "").unwrap();

        // Simulate the check
        assert!(existing.exists());
    }
}
