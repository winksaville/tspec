use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Directory containing fixture projects.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Copy a fixture project into a fresh temporary directory.
///
/// Returns `(TempDir, PathBuf)` — the temp dir guard (drop removes it) and the
/// path to the copied project root inside the temp dir.
pub fn copy_fixture(name: &str) -> (TempDir, PathBuf) {
    let src = fixtures_dir().join(name);
    assert!(src.is_dir(), "fixture not found: {}", src.display());

    let tmp = TempDir::new().expect("failed to create temp dir");
    let dst = tmp.path().join(name);
    copy_dir_recursive(&src, &dst).expect("failed to copy fixture");
    (tmp, dst)
}

/// Recursively copy a directory, skipping `target/` subdirectories.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if file_name == "target" {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&file_name);
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
