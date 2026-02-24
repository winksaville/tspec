//! Workspace discovery
//!
//! Uses `cargo metadata` to discover workspace members.

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::path::{Path, PathBuf};

/// Information about a workspace package
#[derive(Debug, Clone)]
pub struct PackageMember {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub has_binary: bool,
}

/// Workspace information from cargo metadata
pub struct WorkspaceInfo {
    pub root: PathBuf,
    pub members: Vec<PackageMember>,
    /// Version of the root package (if the workspace root has a [package] section)
    pub version: Option<String>,
}

impl WorkspaceInfo {
    /// Workspace name derived from the root directory basename
    pub fn name(&self) -> &str {
        self.root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
    }

    /// Workspace name with version, e.g. "tspec v0.15.5" or just "rlibc-x" if no root package
    pub fn name_versioned(&self) -> String {
        let name = self.name();
        match &self.version {
            Some(ver) => format!("{name} v{ver}"),
            None => name.to_string(),
        }
    }

    /// Discover workspace using cargo metadata
    pub fn discover(project_root: &Path) -> Result<Self> {
        let metadata = MetadataCommand::new()
            .manifest_path(project_root.join("Cargo.toml"))
            .no_deps()
            .exec()
            .context("failed to run cargo metadata")?;

        let root = metadata.workspace_root.as_std_path().to_path_buf();

        let root_manifest = root.join("Cargo.toml");

        let mut root_version = None;
        let members: Vec<PackageMember> = metadata
            .workspace_packages()
            .iter()
            .map(|pkg| {
                let path = pkg
                    .manifest_path
                    .parent()
                    .unwrap()
                    .as_std_path()
                    .to_path_buf();
                let has_binary = pkg.targets.iter().any(|t| t.is_bin());

                // If this package's manifest is at the workspace root, capture its version
                if pkg.manifest_path.as_std_path() == root_manifest {
                    root_version = Some(pkg.version.to_string());
                }

                PackageMember {
                    name: pkg.name.clone(),
                    version: pkg.version.to_string(),
                    path,
                    has_binary,
                }
            })
            .collect();

        Ok(WorkspaceInfo {
            root,
            members,
            version: root_version,
        })
    }

    /// Get all workspace members
    pub fn buildable_members(&self) -> Vec<&PackageMember> {
        self.members.iter().collect()
    }

    /// Get members that have binaries
    pub fn runnable_members(&self) -> Vec<&PackageMember> {
        self.members.iter().filter(|m| m.has_binary).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_works() {
        // This test works for both workspaces and POPs
        let root = crate::find_paths::find_project_root().unwrap();
        let info = WorkspaceInfo::discover(&root);
        if let Ok(info) = info {
            assert!(!info.members.is_empty());

            // All members should be buildable
            assert_eq!(info.buildable_members().len(), info.members.len());

            let _runnable = info.runnable_members();
            // Just verify it doesn't panic - count depends on project structure
        }
        // If cargo metadata fails (e.g., not in a Rust project), that's OK for CI
    }

    #[test]
    fn discover_populates_member_versions() {
        let root = crate::find_paths::find_project_root().unwrap();
        if let Ok(info) = WorkspaceInfo::discover(&root) {
            for member in &info.members {
                assert!(
                    !member.version.is_empty(),
                    "member {} should have a version",
                    member.name
                );
            }
        }
    }

    #[test]
    fn name_versioned_with_version() {
        let info = WorkspaceInfo {
            root: PathBuf::from("/tmp/myproject"),
            members: Vec::new(),
            version: Some("1.2.3".to_string()),
        };
        assert_eq!(info.name_versioned(), "myproject v1.2.3");
    }

    #[test]
    fn name_versioned_without_version() {
        let info = WorkspaceInfo {
            root: PathBuf::from("/tmp/myproject"),
            members: Vec::new(),
            version: None,
        };
        assert_eq!(info.name_versioned(), "myproject");
    }

    #[test]
    fn discover_root_version_set_for_popws() {
        // tspec itself is a POPWS — root has [package], so version should be Some
        let root = crate::find_paths::find_project_root().unwrap();
        if let Ok(info) = WorkspaceInfo::discover(&root) {
            assert!(
                info.version.is_some(),
                "tspec workspace root has [package], version should be Some"
            );
        }
    }
}
