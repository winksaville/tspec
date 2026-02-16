use std::path::PathBuf;
use tspec::options::PanicMode;
use tspec::tspec::{hash_spec, load_spec};
use tspec::types::*;

fn test_data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

#[test]
fn load_minimal_spec() {
    let spec = load_spec(&test_data("minimal.toml")).unwrap();

    assert_eq!(spec.panic, Some(PanicMode::Abort));

    assert_eq!(spec.cargo.profile, Some(Profile::Release));
    assert!(spec.cargo.target_triple.is_none());
    assert!(spec.cargo.target_json.is_none());
    assert!(spec.cargo.unstable.is_empty());

    assert_eq!(spec.rustc.opt_level, Some(OptLevel::Oz));
    assert!(spec.rustc.lto.is_none());
    assert!(spec.rustc.codegen_units.is_none());
    assert!(spec.rustc.build_std.is_empty());
    assert!(spec.rustc.flags.is_empty());

    assert_eq!(spec.linker, LinkerConfig::default());
}

#[test]
fn load_spec_file_not_found() {
    let result = load_spec(&test_data("nonexistent.toml"));
    assert!(result.is_err());
}

#[test]
fn loaded_spec_hash_is_stable() {
    let spec = load_spec(&test_data("minimal.toml")).unwrap();
    let hash1 = hash_spec(&spec).unwrap();
    let hash2 = hash_spec(&spec).unwrap();
    assert_eq!(hash1, hash2);
}

#[test]
fn load_ex_x1_spec() {
    let spec = load_spec(&test_data("ex-x1.ts.toml")).unwrap();

    assert_eq!(spec.cargo, CargoConfig::default());
    assert_eq!(spec.rustc, RustcConfig::default());
    assert_eq!(
        spec.linker.args,
        vec!["-static".to_string(), "-nostartfiles".to_string()]
    );
    assert!(spec.linker.version_script.is_none());
}

#[test]
fn load_ex_x2_spec() {
    let spec = load_spec(&test_data("ex-x2.ts.toml")).unwrap();

    assert_eq!(spec.cargo, CargoConfig::default());
    assert_eq!(spec.rustc, RustcConfig::default());
    assert_eq!(
        spec.linker.args,
        vec![
            "-static".to_string(),
            "-nostdlib".to_string(),
            "-nodefaultlibs".to_string(),
            "-e_start".to_string(),
            "-Wl,--undefined=_start".to_string(),
            "-Wl,--undefined=__libc_start_main".to_string(),
        ]
    );
    assert!(spec.linker.version_script.is_none());
}
