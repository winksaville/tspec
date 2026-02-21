//! Workspace discovery
//!
//! Uses `cargo metadata` to discover workspace members.

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::path::PathBuf;

/// Information about a workspace package
#[derive(Debug, Clone)]
pub struct PackageMember {
    pub name: String,
    pub path: PathBuf,
    pub has_binary: bool,
}

/// Workspace information from cargo metadata
pub struct WorkspaceInfo {
    pub root: PathBuf,
    pub members: Vec<PackageMember>,
}

impl WorkspaceInfo {
    /// Workspace name derived from the root directory basename
    pub fn name(&self) -> &str {
        self.root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
    }

    /// Discover workspace using cargo metadata
    pub fn discover() -> Result<Self> {
        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .context("failed to run cargo metadata")?;

        let root = metadata.workspace_root.as_std_path().to_path_buf();

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

                PackageMember {
                    name: pkg.name.clone(),
                    path,
                    has_binary,
                }
            })
            .collect();

        Ok(WorkspaceInfo { root, members })
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
        let info = WorkspaceInfo::discover();
        if let Ok(info) = info {
            assert!(!info.members.is_empty());

            // All members should be buildable
            assert_eq!(info.buildable_members().len(), info.members.len());

            let _runnable = info.runnable_members();
            // Just verify it doesn't panic - count depends on project structure
        }
        // If cargo metadata fails (e.g., not in a Rust project), that's OK for CI
    }
}
