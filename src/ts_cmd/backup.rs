//! `tspec ts backup` - Create a versioned backup of a tspec (byte-for-byte copy)

use anyhow::{Result, bail};
use std::path::Path;

use crate::find_paths::{find_tspec, resolve_package_dir};
use crate::tspec::{copy_spec_snapshot, spec_name_from_path};

/// Create a versioned backup snapshot of a tspec
pub fn backup_tspec(project_root: &Path, package: Option<&str>, tspec: Option<&str>) -> Result<()> {
    let workspace = project_root;
    let package_dir = resolve_package_dir(workspace, package)?;

    let spec_path = match find_tspec(&package_dir, tspec)? {
        Some(path) => path,
        None => bail!("no tspec found to backup"),
    };

    let base_name = spec_name_from_path(&spec_path);
    let backup_path = copy_spec_snapshot(&spec_path, &base_name, &package_dir)?;

    println!(
        "Backed up to {}",
        backup_path
            .strip_prefix(workspace)
            .unwrap_or(&backup_path)
            .display()
    );

    Ok(())
}
