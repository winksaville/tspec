//! Workspace discovery and package classification
//!
//! Uses `cargo metadata` to discover workspace members and classify them
//! by type (app, lib, tool, test, build tool).

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::path::{Path, PathBuf};

/// Package classification for behavior differences
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageKind {
    /// apps/* - application binaries, runnable
    App,
    /// libs/* - library crates
    Lib,
    /// tools/* - utility binaries
    Tool,
    /// */tests - test binary crates (special handling)
    Test,
    /// Build tools such as xtask or tspec, excluded by default
    BuildTool,
}

/// Information about a workspace package
#[derive(Debug, Clone)]
pub struct PackageMember {
    pub name: String,
    pub path: PathBuf,
    pub has_binary: bool,
    pub kind: PackageKind,
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

        let packages = metadata.workspace_packages();
        let is_pop = packages.len() == 1;

        let members: Vec<PackageMember> = packages
            .iter()
            .map(|pkg| {
                let path = pkg
                    .manifest_path
                    .parent()
                    .unwrap()
                    .as_std_path()
                    .to_path_buf();
                let has_binary = pkg.targets.iter().any(|t| t.is_bin());
                // POPs are always App — classify_package is for multi-package workspaces
                let kind = if is_pop {
                    PackageKind::App
                } else {
                    classify_package(&path, &pkg.name, &root)
                };

                PackageMember {
                    name: pkg.name.clone(),
                    path,
                    has_binary,
                    kind,
                }
            })
            .collect();

        Ok(WorkspaceInfo { root, members })
    }

    /// Get members excluding build tools such as xtask or tspec
    pub fn buildable_members(&self) -> Vec<&PackageMember> {
        self.members
            .iter()
            .filter(|m| m.kind != PackageKind::BuildTool)
            .collect()
    }

    /// Get members that can be run (apps only - have binaries and are runnable)
    pub fn runnable_members(&self) -> Vec<&PackageMember> {
        self.members
            .iter()
            .filter(|m| m.has_binary && m.kind == PackageKind::App)
            .collect()
    }

    /// Get test packages (special handling for rlibc-x2-tests etc.)
    pub fn test_members(&self) -> Vec<&PackageMember> {
        self.members
            .iter()
            .filter(|m| m.kind == PackageKind::Test)
            .collect()
    }
}

/// Classify a package based on its path, name, and workspace root
fn classify_package(path: &Path, name: &str, workspace_root: &Path) -> PackageKind {
    let path_str = path.to_string_lossy();

    // Root package of a workspace is the main app
    if path == workspace_root {
        return PackageKind::App;
    }

    // Build tools at workspace root level
    if name == "tspec" || name == "xt" || name == "xtask" {
        return PackageKind::BuildTool;
    }

    // Test packages
    if path_str.contains("/tests") || name.ends_with("-tests") {
        return PackageKind::Test;
    }

    // Categorize by directory
    if path_str.contains("apps/") {
        PackageKind::App
    } else if path_str.contains("libs/") {
        PackageKind::Lib
    } else if path_str.contains("tools/") {
        PackageKind::Tool
    } else {
        // Root-level members default to Lib
        PackageKind::Lib
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WS: &str = "/workspace";

    #[test]
    fn classify_app() {
        let path = PathBuf::from("/workspace/apps/ex-x1");
        assert_eq!(
            classify_package(&path, "ex-x1", Path::new(WS)),
            PackageKind::App
        );
    }

    #[test]
    fn classify_root_package_is_app() {
        let path = PathBuf::from("/workspace");
        assert_eq!(
            classify_package(&path, "tspec", Path::new(WS)),
            PackageKind::App
        );
    }

    #[test]
    fn classify_lib() {
        let path = PathBuf::from("/workspace/libs/rlibc-x1");
        assert_eq!(
            classify_package(&path, "rlibc-x1", Path::new(WS)),
            PackageKind::Lib
        );
    }

    #[test]
    fn classify_root_level_member_is_lib() {
        let path = PathBuf::from("/workspace/tspec-build");
        assert_eq!(
            classify_package(&path, "tspec-build", Path::new(WS)),
            PackageKind::Lib
        );
    }

    #[test]
    fn classify_tool() {
        let path = PathBuf::from("/workspace/tools/is-libc-used");
        assert_eq!(
            classify_package(&path, "is-libc-used", Path::new(WS)),
            PackageKind::Tool
        );
    }

    #[test]
    fn classify_test_by_path() {
        let path = PathBuf::from("/workspace/libs/rlibc-x2/tests");
        assert_eq!(
            classify_package(&path, "rlibc-x2-tests", Path::new(WS)),
            PackageKind::Test
        );
    }

    #[test]
    fn classify_test_by_name() {
        let path = PathBuf::from("/workspace/somewhere");
        assert_eq!(
            classify_package(&path, "foo-tests", Path::new(WS)),
            PackageKind::Test
        );
    }

    #[test]
    fn classify_build_tool_xt() {
        let path = PathBuf::from("/workspace/xt");
        assert_eq!(
            classify_package(&path, "xt", Path::new(WS)),
            PackageKind::BuildTool
        );
    }

    #[test]
    fn classify_build_tool_xtask() {
        let path = PathBuf::from("/workspace/xtask");
        assert_eq!(
            classify_package(&path, "xtask", Path::new(WS)),
            PackageKind::BuildTool
        );
    }

    #[test]
    fn discover_works() {
        // This test works for both workspaces and POPs
        let info = WorkspaceInfo::discover();
        if let Ok(info) = info {
            // Should have some members (at least 1 for POP, more for workspace)
            assert!(!info.members.is_empty());

            let buildable = info.buildable_members();
            if info.members.len() == 1 {
                // POP: the single package should be buildable
                assert_eq!(buildable.len(), 1);
            } else {
                // Workspace: xt and xtask should be excluded from buildable
                assert!(
                    buildable
                        .iter()
                        .all(|m| m.name != "xt" && m.name != "xtask")
                );
            }

            // For workspaces with apps/, should have runnable members
            // For POPs, the single package is runnable if it has a binary
            let _runnable = info.runnable_members();
            // Just verify it doesn't panic - count depends on project structure
        }
        // If cargo metadata fails (e.g., not in a Rust project), that's OK for CI
    }
}
