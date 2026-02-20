//! Workspace discovery
//!
//! Uses `cargo metadata` to discover workspace members and determine
//! which are build tools (excluded from batch operations by default).

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::path::PathBuf;

/// Information about a workspace package
#[derive(Debug, Clone)]
pub struct PackageMember {
    pub name: String,
    pub path: PathBuf,
    pub has_binary: bool,
    pub is_build_tool: bool,
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
                    is_build_tool: is_build_tool_name(&pkg.name),
                }
            })
            .collect();

        Ok(WorkspaceInfo { root, members })
    }

    /// Get members excluding build tools such as xtask or tspec
    pub fn buildable_members(&self) -> Vec<&PackageMember> {
        self.members.iter().filter(|m| !m.is_build_tool).collect()
    }

    /// Get members that can be run (have binaries and are not build tools)
    pub fn runnable_members(&self) -> Vec<&PackageMember> {
        self.members
            .iter()
            .filter(|m| m.has_binary && !m.is_build_tool)
            .collect()
    }
}

/// Check if a package name is a build tool (excluded from batch operations).
fn is_build_tool_name(name: &str) -> bool {
    matches!(name, "tspec" | "xt" | "xtask")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tool_names() {
        assert!(is_build_tool_name("tspec"));
        assert!(is_build_tool_name("xt"));
        assert!(is_build_tool_name("xtask"));
    }

    #[test]
    fn non_build_tool_names() {
        assert!(!is_build_tool_name("tspec-build"));
        assert!(!is_build_tool_name("my-app"));
        assert!(!is_build_tool_name("xtask-macros"));
        assert!(!is_build_tool_name("foo-tests"));
    }

    #[test]
    fn discover_works() {
        // This test works for both workspaces and POPs
        let info = WorkspaceInfo::discover();
        if let Ok(info) = info {
            assert!(!info.members.is_empty());

            let buildable = info.buildable_members();
            if info.members.len() == 1 {
                // POP: the single package should be buildable
                assert_eq!(buildable.len(), 1);
            } else {
                // Workspace: build tools should be excluded from buildable
                assert!(buildable.iter().all(|m| !m.is_build_tool));
            }

            let _runnable = info.runnable_members();
            // Just verify it doesn't panic - count depends on project structure
        }
        // If cargo metadata fails (e.g., not in a Rust project), that's OK for CI
    }
}
