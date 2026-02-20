use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;

use crate::options::{PanicMode, StripMode};

/// Verbosity level for command output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    #[default]
    Normal,
    /// -v: print command line + env vars, pass -v to cargo
    Verbose,
    /// -vv: also print spec resolution details, pass -vv to cargo
    Debug,
}

impl Verbosity {
    pub fn from_count(count: u8) -> Self {
        match count {
            0 => Verbosity::Normal,
            1 => Verbosity::Verbose,
            _ => Verbosity::Debug,
        }
    }
}

/// Global flags passed through to cargo commands.
///
/// Collected once from the CLI and threaded through all command execution.
/// Adding a new passthrough flag only requires adding a field here and
/// handling it in `apply_to_command()`.
#[derive(Debug, Clone, Default)]
pub struct CargoFlags {
    pub verbosity: Verbosity,
    /// Number of parallel jobs (-j N)
    pub jobs: Option<u16>,
    /// Extra args appended to the cargo command (e.g., `["--test", "name", "--", "--ignored"]`)
    pub extra_args: Vec<String>,
}

impl CargoFlags {
    /// Apply these flags to a cargo command.
    pub fn apply_to_command(&self, cmd: &mut Command) {
        match self.verbosity {
            Verbosity::Verbose => {
                cmd.arg("-v");
            }
            Verbosity::Debug => {
                cmd.arg("-vv");
            }
            Verbosity::Normal => {}
        }
        if let Some(j) = self.jobs {
            cmd.arg("-j").arg(j.to_string());
        }
        if !self.extra_args.is_empty() {
            cmd.args(&self.extra_args);
        }
    }
}

/// A value in the `[cargo.config]` table.
/// Uses `#[serde(untagged)]` so TOML bools/ints/strings/tables are deserialized naturally.
/// We avoid `toml::Value` because it contains `Float(f64)` which doesn't implement `Eq`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Bool(bool),
    Integer(i64),
    String(String),
    Table(BTreeMap<String, ConfigValue>),
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigValue::Bool(b) => write!(f, "{}", b),
            ConfigValue::Integer(n) => write!(f, "{}", n),
            ConfigValue::String(s) => write!(f, "\"{}\"", s),
            ConfigValue::Table(map) => write!(f, "{:?}", map),
        }
    }
}

/// Flatten nested config into dotted key-value pairs for --config args.
pub fn flatten_config(config: &BTreeMap<String, ConfigValue>) -> Vec<(String, String)> {
    let mut result = Vec::new();
    flatten_inner(config, String::new(), &mut result);
    result
}

fn flatten_inner(
    map: &BTreeMap<String, ConfigValue>,
    prefix: String,
    result: &mut Vec<(String, String)>,
) {
    for (key, value) in map {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };
        match value {
            ConfigValue::Table(inner) => flatten_inner(inner, full_key, result),
            other => result.push((full_key, other.to_string())),
        }
    }
}

/// Map a profile name to the directory cargo uses in target/.
/// `"dev"` → `"debug"`, `"release"` → `"release"`, custom → as-is.
pub fn profile_dir_name(profile: &str) -> &str {
    match profile {
        "dev" => "debug",
        other => other,
    }
}

/// Cargo-specific configuration (flat struct)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoConfig {
    /// Build profile name (e.g., "debug", "release", "release-small", or any custom profile)
    pub profile: Option<String>,
    /// Target triple (e.g., "x86_64-unknown-linux-musl")
    pub target_triple: Option<String>,
    /// Custom target JSON file path
    pub target_json: Option<PathBuf>,
    /// Nightly-only -Z flags (e.g., ["panic-immediate-abort"])
    #[serde(default)]
    pub unstable: Vec<String>,
    /// Custom target directory subdirectory for per-spec isolation.
    /// Supports `{name}` (spec filename sans .ts.toml) and `{hash}` (8-char content hash).
    pub target_dir: Option<String>,
    /// Config values passed as `--config 'KEY=VALUE'` to cargo.
    /// Supports both flat dotted keys and nested tables.
    /// Each leaf entry becomes a separate `--config` arg.
    #[serde(default)]
    pub config: BTreeMap<String, ConfigValue>,
    /// Crates to rebuild with -Z build-std (nightly only)
    #[serde(default)]
    pub build_std: Vec<String>,
}

/// Version script configuration for symbol visibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionScript {
    /// Symbols to keep global (e.g., ["_start"])
    pub global: Vec<String>,
    /// Pattern for local symbols (typically "*")
    #[serde(default = "default_local")]
    pub local: String,
}

fn default_local() -> String {
    "*".to_string()
}

/// Linker configuration (flat struct)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkerConfig {
    /// Linker arguments (e.g., ["-static", "-nostdlib"])
    #[serde(default)]
    pub args: Vec<String>,
    /// Version script for symbol visibility (enables --gc-sections optimization)
    pub version_script: Option<VersionScript>,
}

/// A translation spec
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spec {
    /// High-level panic mode (sets both cargo -Z and rustc -C flags)
    pub panic: Option<PanicMode>,

    /// High-level strip mode (sets rustc -C strip=)
    pub strip: Option<StripMode>,

    /// Rust toolchain override (e.g., "nightly", "stable", "nightly-2025-01-15", "1.75")
    pub toolchain: Option<String>,

    #[serde(default)]
    pub cargo: CargoConfig,
    /// Raw flags passed through to RUSTFLAGS
    #[serde(default)]
    pub rustflags: Vec<String>,
    #[serde(default)]
    pub linker: LinkerConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_default_is_empty() {
        let spec = Spec::default();
        assert_eq!(spec.cargo, CargoConfig::default());
        assert!(spec.rustflags.is_empty());
        assert_eq!(spec.linker, LinkerConfig::default());
    }

    #[test]
    fn profile_dir_name_dev_maps_to_debug() {
        assert_eq!(profile_dir_name("dev"), "debug");
    }

    #[test]
    fn profile_dir_name_release_unchanged() {
        assert_eq!(profile_dir_name("release"), "release");
    }

    #[test]
    fn profile_dir_name_custom_unchanged() {
        assert_eq!(profile_dir_name("release-small"), "release-small");
    }

    #[test]
    fn flatten_config_nested() {
        let config = BTreeMap::from([(
            "profile".to_string(),
            ConfigValue::Table(BTreeMap::from([(
                "release".to_string(),
                ConfigValue::Table(BTreeMap::from([
                    (
                        "opt-level".to_string(),
                        ConfigValue::String("z".to_string()),
                    ),
                    ("codegen-units".to_string(), ConfigValue::Integer(1)),
                ])),
            )])),
        )]);
        let flat = flatten_config(&config);
        assert!(flat.contains(&("profile.release.codegen-units".to_string(), "1".to_string())));
        assert!(flat.contains(&("profile.release.opt-level".to_string(), "\"z\"".to_string())));
        assert_eq!(flat.len(), 2);
    }

    #[test]
    fn flatten_config_empty() {
        let config = BTreeMap::new();
        assert!(flatten_config(&config).is_empty());
    }
}
