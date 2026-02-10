//! `tspec ts restore` - Restore a tspec from a versioned backup (byte-for-byte copy)

use anyhow::{Context, Result, bail};
use std::path::Path;

use crate::TSPEC_SUFFIX;
use crate::find_paths::{find_tspec, resolve_package_dir};

/// Restore a tspec from a versioned backup to its base name
pub fn restore_tspec(project_root: &Path, package: Option<&str>, tspec: &str) -> Result<()> {
    let workspace = project_root;
    let package_dir = resolve_package_dir(workspace, package)?;

    let backup_path = match find_tspec(&package_dir, Some(tspec))? {
        Some(path) => path,
        None => bail!("backup tspec not found: {}", tspec),
    };

    let base_name = parse_backup_base_name(&backup_path)?;
    let target_path = package_dir.join(format!("{}{}", base_name, TSPEC_SUFFIX));

    std::fs::copy(&backup_path, &target_path).with_context(|| {
        format!(
            "failed to copy {} to {}",
            backup_path.display(),
            target_path.display()
        )
    })?;

    println!(
        "Restored {} from {}",
        target_path
            .strip_prefix(workspace)
            .unwrap_or(&target_path)
            .display(),
        backup_path
            .strip_prefix(workspace)
            .unwrap_or(&backup_path)
            .display()
    );

    Ok(())
}

/// Parse a backup filename to extract the base name.
/// Backup filenames have the pattern `{base}-{NNN}-{HHHHHHHH}.ts.toml`
/// where NNN is a 3-digit sequence and HHHHHHHH is an 8-char hex hash.
fn parse_backup_base_name(path: &Path) -> Result<String> {
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let stem = filename
        .strip_suffix(TSPEC_SUFFIX)
        .or_else(|| filename.strip_suffix(".toml"))
        .unwrap_or(&filename);

    // Look for trailing -{NNN}-{HHHHHHHH} pattern
    // Split from the right: last segment is hash (8 hex chars), second-to-last is seq (3 digits)
    let parts: Vec<&str> = stem.rsplitn(3, '-').collect();
    if parts.len() == 3 {
        let hash_part = parts[0];
        let seq_part = parts[1];
        let base_part = parts[2];

        if seq_part.len() == 3
            && seq_part.chars().all(|c| c.is_ascii_digit())
            && hash_part.len() == 8
            && hash_part.chars().all(|c| c.is_ascii_hexdigit())
            && !base_part.is_empty()
        {
            return Ok(base_part.to_string());
        }
    }

    bail!(
        "not a backup filename (expected {{name}}-NNN-HHHHHHHH{}): {}",
        TSPEC_SUFFIX,
        filename
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_simple_backup_name() {
        let path = PathBuf::from("/dir/t2-001-abcd1234.ts.toml");
        assert_eq!(parse_backup_base_name(&path).unwrap(), "t2");
    }

    #[test]
    fn parse_backup_name_with_hyphens() {
        let path = PathBuf::from("/dir/my-spec-name-003-deadbeef.ts.toml");
        assert_eq!(parse_backup_base_name(&path).unwrap(), "my-spec-name");
    }

    #[test]
    fn parse_backup_name_with_dots() {
        let path = PathBuf::from("/dir/tspec.static-opt-001-12345678.ts.toml");
        assert_eq!(parse_backup_base_name(&path).unwrap(), "tspec.static-opt");
    }

    #[test]
    fn reject_non_backup_name() {
        let path = PathBuf::from("/dir/t2.ts.toml");
        assert!(parse_backup_base_name(&path).is_err());
    }

    #[test]
    fn reject_wrong_seq_length() {
        let path = PathBuf::from("/dir/t2-01-abcd1234.ts.toml");
        assert!(parse_backup_base_name(&path).is_err());
    }

    #[test]
    fn reject_wrong_hash_length() {
        let path = PathBuf::from("/dir/t2-001-abcd.ts.toml");
        assert!(parse_backup_base_name(&path).is_err());
    }

    #[test]
    fn reject_non_hex_hash() {
        let path = PathBuf::from("/dir/t2-001-ghijklmn.ts.toml");
        assert!(parse_backup_base_name(&path).is_err());
    }
}
