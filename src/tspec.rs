use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::TSPEC_SUFFIX;
use crate::types::{Spec, validate_config_profiles};

/// Load a spec from a TOML file
pub fn load_spec(path: &Path) -> Result<Spec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read spec file: {}", path.display()))?;
    let spec = parse_spec(&content)?;
    if let Err(msg) = validate_config_profiles(&spec.cargo.config) {
        anyhow::bail!("{}: {}", path.display(), msg);
    }
    Ok(spec)
}

/// Parse a spec from TOML string
pub fn parse_spec(toml_str: &str) -> Result<Spec> {
    toml::from_str(toml_str).context("failed to parse spec TOML")
}

/// Serialize a spec to TOML string (canonical form for hashing)
pub fn serialize_spec(spec: &Spec) -> Result<String> {
    toml::to_string(spec).context("failed to serialize spec")
}

/// Compute hash of a resolved spec (first 8 hex chars)
pub fn hash_spec(spec: &Spec) -> Result<String> {
    let toml_str = serialize_spec(spec)?;
    let mut hasher = Sha256::new();
    hasher.update(toml_str.as_bytes());
    let result = hasher.finalize();
    Ok(hex::encode(&result[..4]))
}

/// Extract spec name from path by stripping the .ts.toml (or .toml) suffix
pub fn spec_name_from_path(path: &Path) -> String {
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    filename
        .strip_suffix(TSPEC_SUFFIX)
        .or_else(|| filename.strip_suffix(".toml"))
        .unwrap_or(&filename)
        .to_string()
}

/// Expand template placeholders in a spec's target_dir field.
/// Returns None if target_dir is absent or empty.
pub fn expand_target_dir(spec: &Spec, spec_name: &str) -> Result<Option<String>> {
    let raw = match &spec.cargo.target_dir {
        Some(td) if !td.is_empty() => td,
        _ => return Ok(None),
    };

    let mut expanded = raw.clone();

    if expanded.contains("{name}") {
        expanded = expanded.replace("{name}", spec_name);
    }

    if expanded.contains("{hash}") {
        let hash = hash_spec(spec)?;
        expanded = expanded.replace("{hash}", &hash);
    }

    if expanded.is_empty() {
        Ok(None)
    } else {
        Ok(Some(expanded))
    }
}

/// Save a spec to a TOML file, creating parent directories if needed
pub fn save_spec(spec: &Spec, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }
    let content = serialize_spec(spec)?;
    std::fs::write(path, &content)
        .with_context(|| format!("failed to write spec file: {}", path.display()))
}

/// Find the next sequence number for snapshot files matching `{name}-NNN-*{TSPEC_SUFFIX}`.
pub fn next_snapshot_seq(name: &str, dir: &Path) -> Result<u32> {
    let prefix = format!("{}-", name);
    let next_seq = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory: {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let filename = entry.file_name().to_string_lossy().into_owned();
            if filename.starts_with(&prefix) && filename.ends_with(TSPEC_SUFFIX) {
                let rest = filename.strip_prefix(&prefix)?;
                let seq_str = rest.split('-').next()?;
                seq_str.parse::<u32>().ok()
            } else {
                None
            }
        })
        .max()
        .map(|n| n + 1)
        .unwrap_or(1);
    Ok(next_seq)
}

/// Save a spec snapshot with sequence number and hash: `{name}-{seq:03}-{hash}.toml`
/// Returns the path of the created file.
pub fn save_spec_snapshot(spec: &Spec, name: &str, dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create directory: {}", dir.display()))?;

    let next_seq = next_snapshot_seq(name, dir)?;
    let hash = hash_spec(spec)?;
    let filename = format!("{}-{:03}-{}{}", name, next_seq, hash, TSPEC_SUFFIX);
    let path = dir.join(&filename);

    let content = serialize_spec(spec)?;
    std::fs::write(&path, &content)
        .with_context(|| format!("failed to write spec file: {}", path.display()))?;

    Ok(path)
}

/// Create a backup snapshot using raw file copy (preserves comments/formatting).
/// Loads the spec only for hashing (to compute the backup filename).
/// Returns the path of the created backup file.
pub fn copy_spec_snapshot(source: &Path, name: &str, dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create directory: {}", dir.display()))?;

    let spec = load_spec(source)?;
    let next_seq = next_snapshot_seq(name, dir)?;
    let hash = hash_spec(&spec)?;
    let filename = format!("{}-{:03}-{}{}", name, next_seq, hash, TSPEC_SUFFIX);
    let path = dir.join(&filename);

    std::fs::copy(source, &path)
        .with_context(|| format!("failed to copy {} to {}", source.display(), path.display()))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_constants::SUFFIX;
    use crate::types::*;

    #[test]
    fn parse_empty_spec() {
        let spec = parse_spec("").unwrap();
        assert_eq!(spec.cargo, CargoConfig::default());
        assert!(spec.rustflags.is_empty());
        assert_eq!(spec.linker, LinkerConfig::default());
    }

    #[test]
    fn hash_is_stable() {
        let spec = Spec::default();
        let hash1 = hash_spec(&spec).unwrap();
        let hash2 = hash_spec(&spec).unwrap();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 8);
    }

    #[test]
    fn different_specs_different_hash() {
        let empty = Spec::default();
        let with_release = Spec {
            cargo: CargoConfig {
                profile: Some(Profile::Release),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_ne!(
            hash_spec(&empty).unwrap(),
            hash_spec(&with_release).unwrap()
        );
    }

    #[test]
    fn save_and_load_roundtrip() {
        let spec = Spec {
            panic: None,
            strip: None,
            cargo: CargoConfig {
                profile: Some(Profile::Release),
                ..Default::default()
            },
            rustflags: vec!["-Cforce-frame-pointers=yes".to_string()],
            linker: LinkerConfig {
                args: vec!["-static".to_string()],
                ..Default::default()
            },
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");

        save_spec(&spec, &path).unwrap();
        let loaded = load_spec(&path).unwrap();

        assert_eq!(spec.cargo, loaded.cargo);
        assert_eq!(spec.rustflags, loaded.rustflags);
        assert_eq!(spec.linker, loaded.linker);
    }

    #[test]
    fn save_spec_creates_parent_dirs() {
        let spec = Spec::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deep").join("test.toml");

        save_spec(&spec, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn snapshot_creates_sequenced_files() {
        let dir = tempfile::tempdir().unwrap();
        let spec1 = Spec::default();
        let spec2 = Spec {
            cargo: CargoConfig {
                profile: Some(Profile::Release),
                ..Default::default()
            },
            ..Default::default()
        };

        let path1 = save_spec_snapshot(&spec1, "test", dir.path()).unwrap();
        let path2 = save_spec_snapshot(&spec2, "test", dir.path()).unwrap();
        let path3 = save_spec_snapshot(&spec1, "test", dir.path()).unwrap();

        let name1 = path1.file_name().unwrap().to_string_lossy();
        let name2 = path2.file_name().unwrap().to_string_lossy();
        let name3 = path3.file_name().unwrap().to_string_lossy();

        assert!(name1.starts_with("test-001-"), "got: {}", name1);
        assert!(name2.starts_with("test-002-"), "got: {}", name2);
        assert!(name3.starts_with("test-003-"), "got: {}", name3);
        assert!(name1.ends_with(SUFFIX), "got: {}", name1);

        // Same content = same hash suffix (strip prefix and SUFFIX)
        let hash1 = name1
            .strip_prefix("test-001-")
            .unwrap()
            .strip_suffix(SUFFIX)
            .unwrap();
        let hash3 = name3
            .strip_prefix("test-003-")
            .unwrap()
            .strip_suffix(SUFFIX)
            .unwrap();
        assert_eq!(hash1, hash3);
    }

    // ==================== spec_name_from_path tests ====================

    #[test]
    fn spec_name_strips_ts_toml_suffix() {
        let path = PathBuf::from("/foo/bar/tspec.static-opt.ts.toml");
        assert_eq!(spec_name_from_path(&path), "tspec.static-opt");
    }

    #[test]
    fn spec_name_strips_plain_toml_suffix() {
        let path = PathBuf::from("/foo/bar/minimal.toml");
        assert_eq!(spec_name_from_path(&path), "minimal");
    }

    #[test]
    fn spec_name_no_known_suffix() {
        let path = PathBuf::from("/foo/bar/weird.txt");
        assert_eq!(spec_name_from_path(&path), "weird.txt");
    }

    // ==================== expand_target_dir tests ====================

    #[test]
    fn expand_target_dir_none() {
        let spec = Spec::default();
        assert_eq!(expand_target_dir(&spec, "foo").unwrap(), None);
    }

    #[test]
    fn expand_target_dir_empty() {
        let mut spec = Spec::default();
        spec.cargo.target_dir = Some("".to_string());
        assert_eq!(expand_target_dir(&spec, "foo").unwrap(), None);
    }

    #[test]
    fn expand_target_dir_literal() {
        let mut spec = Spec::default();
        spec.cargo.target_dir = Some("my-subdir".to_string());
        assert_eq!(
            expand_target_dir(&spec, "foo").unwrap(),
            Some("my-subdir".to_string())
        );
    }

    #[test]
    fn expand_target_dir_name_placeholder() {
        let mut spec = Spec::default();
        spec.cargo.target_dir = Some("{name}".to_string());
        assert_eq!(
            expand_target_dir(&spec, "static-opt").unwrap(),
            Some("static-opt".to_string())
        );
    }

    #[test]
    fn expand_target_dir_hash_placeholder() {
        let mut spec = Spec::default();
        spec.cargo.target_dir = Some("{hash}".to_string());
        let result = expand_target_dir(&spec, "foo").unwrap().unwrap();
        assert_eq!(result.len(), 8);
        assert!(result.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn expand_target_dir_name_and_hash() {
        let mut spec = Spec::default();
        spec.cargo.target_dir = Some("{name}-{hash}".to_string());
        let result = expand_target_dir(&spec, "opt").unwrap().unwrap();
        assert!(result.starts_with("opt-"));
        assert_eq!(result.len(), 4 + 8); // "opt-" + 8-char hash
    }
}
