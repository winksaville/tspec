use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use crate::options::{PanicMode, StripMode};

/// Build profile - mutually exclusive
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Debug,
    Release,
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

/// Validate that config profile keys are only "debug" or "release".
/// Returns an error if e.g. `profile.custom` is found.
pub fn validate_config_profiles(config: &BTreeMap<String, ConfigValue>) -> Result<(), String> {
    if let Some(ConfigValue::Table(profiles)) = config.get("profile") {
        for name in profiles.keys() {
            if name != "debug" && name != "release" {
                return Err(format!(
                    "unsupported profile in [cargo.config]: \"{}\" (only \"debug\" and \"release\" are supported)",
                    name
                ));
            }
        }
    }
    // Also check flat dotted keys like "profile.foo.opt-level"
    for key in config.keys() {
        if let Some(rest) = key.strip_prefix("profile.") {
            let profile_name = rest.split('.').next().unwrap_or(rest);
            if profile_name != "debug" && profile_name != "release" {
                return Err(format!(
                    "unsupported profile in [cargo.config]: \"{}\" (only \"debug\" and \"release\" are supported)",
                    profile_name
                ));
            }
        }
    }
    Ok(())
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

/// Cargo-specific configuration (flat struct)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoConfig {
    /// Build profile (debug or release)
    pub profile: Option<Profile>,
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
    fn validate_config_profiles_accepts_release() {
        let config = BTreeMap::from([(
            "profile".to_string(),
            ConfigValue::Table(BTreeMap::from([(
                "release".to_string(),
                ConfigValue::Table(BTreeMap::from([(
                    "opt-level".to_string(),
                    ConfigValue::String("z".to_string()),
                )])),
            )])),
        )]);
        assert!(validate_config_profiles(&config).is_ok());
    }

    #[test]
    fn validate_config_profiles_accepts_debug() {
        let config = BTreeMap::from([(
            "profile".to_string(),
            ConfigValue::Table(BTreeMap::from([(
                "debug".to_string(),
                ConfigValue::Table(BTreeMap::from([(
                    "opt-level".to_string(),
                    ConfigValue::Integer(2),
                )])),
            )])),
        )]);
        assert!(validate_config_profiles(&config).is_ok());
    }

    #[test]
    fn validate_config_profiles_rejects_custom_nested() {
        let config = BTreeMap::from([(
            "profile".to_string(),
            ConfigValue::Table(BTreeMap::from([(
                "custom".to_string(),
                ConfigValue::Table(BTreeMap::from([(
                    "opt-level".to_string(),
                    ConfigValue::Integer(3),
                )])),
            )])),
        )]);
        let err = validate_config_profiles(&config).unwrap_err();
        assert!(err.contains("custom"));
    }

    #[test]
    fn validate_config_profiles_rejects_custom_flat() {
        let config = BTreeMap::from([(
            "profile.custom.opt-level".to_string(),
            ConfigValue::String("z".to_string()),
        )]);
        let err = validate_config_profiles(&config).unwrap_err();
        assert!(err.contains("custom"));
    }

    #[test]
    fn validate_config_profiles_accepts_non_profile_keys() {
        let config = BTreeMap::from([(
            "build".to_string(),
            ConfigValue::Table(BTreeMap::from([(
                "rustflags".to_string(),
                ConfigValue::String("-C target-cpu=native".to_string()),
            )])),
        )]);
        assert!(validate_config_profiles(&config).is_ok());
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
