//! toml_edit helpers for surgical tspec editing that preserves comments/formatting.

use anyhow::{Result, bail};
use toml_edit::{Array, DocumentMut, Item, Value};

/// Whether a field holds a scalar or an array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Scalar,
    Array,
}

/// The set operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    /// Replace the field (`=`)
    Replace,
    /// Append to an array (`+=`)
    Append,
    /// Remove from an array (`-=`)
    Remove,
}

/// Registry entry: (dotted key path, kind).
const FIELD_REGISTRY: &[(&str, FieldKind)] = &[
    ("panic", FieldKind::Scalar),
    ("strip", FieldKind::Scalar),
    ("cargo.profile", FieldKind::Scalar),
    ("cargo.target_triple", FieldKind::Scalar),
    ("cargo.target_json", FieldKind::Scalar),
    ("cargo.target_dir", FieldKind::Scalar),
    ("cargo.unstable", FieldKind::Array),
    ("rustc.opt_level", FieldKind::Scalar),
    ("rustc.panic", FieldKind::Scalar),
    ("rustc.lto", FieldKind::Scalar),
    ("rustc.codegen_units", FieldKind::Scalar),
    ("rustc.build_std", FieldKind::Array),
    ("rustc.flags", FieldKind::Array),
    ("linker.args", FieldKind::Array),
];

/// Validate that a key is in the registry and return its kind.
pub fn validate_key(key: &str) -> Result<FieldKind> {
    for &(k, kind) in FIELD_REGISTRY {
        if k == key {
            return Ok(kind);
        }
    }
    let valid_keys: Vec<&str> = FIELD_REGISTRY.iter().map(|(k, _)| *k).collect();
    bail!(
        "unknown key: {} (valid keys: {})",
        key,
        valid_keys.join(", ")
    )
}

/// Validate a value for enum-constrained fields.
/// For unconstrained fields (strings, arrays), accepts anything.
pub fn validate_value(key: &str, value: &str) -> Result<()> {
    match key {
        "panic" => match value {
            "unwind" | "abort" | "immediate-abort" => Ok(()),
            _ => bail!(
                "invalid panic mode: {} (expected: unwind, abort, immediate-abort)",
                value
            ),
        },
        "strip" => match value {
            "none" | "debuginfo" | "symbols" => Ok(()),
            _ => bail!(
                "invalid strip mode: {} (expected: none, debuginfo, symbols)",
                value
            ),
        },
        "cargo.profile" => match value {
            "debug" | "release" => Ok(()),
            _ => bail!("invalid profile: {} (expected: debug, release)", value),
        },
        "rustc.opt_level" => match value {
            "0" | "1" | "2" | "3" | "s" | "z" => Ok(()),
            _ => bail!("invalid opt-level: {} (expected: 0, 1, 2, 3, s, z)", value),
        },
        "rustc.panic" => match value {
            "abort" | "unwind" | "immediate-abort" => Ok(()),
            _ => bail!(
                "invalid panic strategy: {} (expected: abort, unwind, immediate-abort)",
                value
            ),
        },
        "rustc.lto" => match value {
            "true" | "false" | "yes" | "no" | "1" | "0" => Ok(()),
            _ => bail!("invalid boolean: {} (expected: true/false)", value),
        },
        "rustc.codegen_units" => {
            value.parse::<u32>().map_err(|_| {
                anyhow::anyhow!("invalid codegen_units: {} (expected integer)", value)
            })?;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Parse a key into (optional table, field).
/// "panic" -> (None, "panic")
/// "rustc.lto" -> (Some("rustc"), "lto")
fn parse_key(key: &str) -> (Option<&str>, &str) {
    match key.split_once('.') {
        Some((table, field)) => (Some(table), field),
        None => (None, key),
    }
}

/// Parse a value string into a toml_edit Value.
/// Booleans -> bool, integers -> i64, everything else -> string.
fn parse_scalar_value(key: &str, raw: &str) -> Value {
    // For rustc.lto, always parse as boolean
    if key == "rustc.lto" {
        return match raw {
            "true" | "yes" | "1" => Value::from(true),
            _ => Value::from(false),
        };
    }

    // For rustc.codegen_units, always parse as integer
    if key == "rustc.codegen_units"
        && let Ok(n) = raw.parse::<i64>()
    {
        return Value::from(n);
    }

    // For rustc.opt_level, keep as string (since "0","1",etc. are enum variants)
    Value::from(raw)
}

/// Parse an array value from a string.
/// With brackets: `["a","b"]` — parsed as TOML inline array (multiple items).
/// Without brackets: treated as a single item (commas are literal).
fn parse_array_value(raw: &str) -> Result<Array> {
    let raw = raw.trim();

    // Bracket syntax: parse as TOML inline array
    if raw.starts_with('[') && raw.ends_with(']') {
        let toml_str = format!("x = {}", raw);
        match toml_str.parse::<DocumentMut>() {
            Ok(doc) => {
                if let Some(Item::Value(Value::Array(arr))) = doc.get("x") {
                    return Ok(arr.clone());
                }
            }
            Err(e) => bail!("invalid array syntax: {} ({})", raw, e),
        }
    }

    // No brackets: single item (commas are literal, e.g. "-Wl,--gc-sections")
    let mut arr = Array::new();
    let item = raw.trim_matches('"').trim_matches('\'');
    if !item.is_empty() {
        arr.push(item);
    }
    Ok(arr)
}

/// Set a field in a toml_edit document.
pub fn set_field(doc: &mut DocumentMut, key: &str, value: &str, kind: FieldKind) -> Result<()> {
    let (table_name, field) = parse_key(key);

    match kind {
        FieldKind::Scalar => {
            let val = parse_scalar_value(key, value);
            match table_name {
                Some(table) => {
                    // Ensure table exists
                    if doc.get(table).is_none() {
                        doc[table] = toml_edit::Item::Table(toml_edit::Table::new());
                    }
                    doc[table][field] = toml_edit::Item::Value(val);
                }
                None => {
                    doc[field] = toml_edit::Item::Value(val);
                }
            }
        }
        FieldKind::Array => {
            let arr = parse_array_value(value)?;
            match table_name {
                Some(table) => {
                    if doc.get(table).is_none() {
                        doc[table] = toml_edit::Item::Table(toml_edit::Table::new());
                    }
                    doc[table][field] = toml_edit::Item::Value(Value::Array(arr));
                }
                None => {
                    doc[field] = toml_edit::Item::Value(Value::Array(arr));
                }
            }
        }
    }

    Ok(())
}

/// Get the existing array for a field, or an empty array if it doesn't exist.
fn get_existing_array(doc: &DocumentMut, key: &str) -> Array {
    let (table_name, field) = parse_key(key);
    match table_name {
        Some(table) => doc
            .get(table)
            .and_then(|t| t.get(field))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        None => doc
            .get(field)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
    }
}

/// Append values to an array field. Creates the array if it doesn't exist.
/// Skips values that are already present (dedup).
pub fn append_field(doc: &mut DocumentMut, key: &str, value: &str) -> Result<()> {
    let (table_name, field) = parse_key(key);
    let mut arr = get_existing_array(doc, key);
    let new_items = parse_array_value(value)?;

    // Collect existing string values for dedup
    let existing: Vec<String> = arr
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    for item in new_items.iter() {
        if let Some(s) = item.as_str()
            && !existing.contains(&s.to_string())
        {
            arr.push(s);
        }
    }

    match table_name {
        Some(table) => {
            if doc.get(table).is_none() {
                doc[table] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            doc[table][field] = toml_edit::Item::Value(Value::Array(arr));
        }
        None => {
            doc[field] = toml_edit::Item::Value(Value::Array(arr));
        }
    }

    Ok(())
}

/// Remove values from an array field. If the array becomes empty, removes the field.
/// If the containing table becomes empty, removes the table too.
pub fn remove_from_field(doc: &mut DocumentMut, key: &str, value: &str) -> Result<()> {
    let (table_name, field) = parse_key(key);
    let arr = get_existing_array(doc, key);
    let to_remove = parse_array_value(value)?;

    let remove_strs: Vec<String> = to_remove
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    let mut new_arr = Array::new();
    for item in arr.iter() {
        if let Some(s) = item.as_str()
            && !remove_strs.contains(&s.to_string())
        {
            new_arr.push(s);
        }
    }

    if new_arr.is_empty() {
        // Remove the field entirely
        unset_field(doc, key)?;
    } else {
        match table_name {
            Some(table) => {
                if doc.get(table).is_none() {
                    doc[table] = toml_edit::Item::Table(toml_edit::Table::new());
                }
                doc[table][field] = toml_edit::Item::Value(Value::Array(new_arr));
            }
            None => {
                doc[field] = toml_edit::Item::Value(Value::Array(new_arr));
            }
        }
    }

    Ok(())
}

/// Remove a field from a toml_edit document.
/// If the containing table becomes empty, remove it too.
pub fn unset_field(doc: &mut DocumentMut, key: &str) -> Result<()> {
    let (table_name, field) = parse_key(key);

    match table_name {
        Some(table) => {
            if let Some(Item::Table(tbl)) = doc.get_mut(table) {
                tbl.remove(field);
                // If table is now empty, remove it
                if tbl.is_empty() {
                    doc.remove(table);
                }
            }
        }
        None => {
            doc.remove(field);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_key_valid_scalar() {
        assert_eq!(validate_key("panic").unwrap(), FieldKind::Scalar);
        assert_eq!(validate_key("rustc.lto").unwrap(), FieldKind::Scalar);
        assert_eq!(validate_key("cargo.profile").unwrap(), FieldKind::Scalar);
    }

    #[test]
    fn validate_key_valid_array() {
        assert_eq!(validate_key("rustc.build_std").unwrap(), FieldKind::Array);
        assert_eq!(validate_key("linker.args").unwrap(), FieldKind::Array);
        assert_eq!(validate_key("cargo.unstable").unwrap(), FieldKind::Array);
        assert_eq!(validate_key("rustc.flags").unwrap(), FieldKind::Array);
    }

    #[test]
    fn validate_key_unknown() {
        let err = validate_key("nonexistent").unwrap_err();
        assert!(err.to_string().contains("unknown key"));
    }

    #[test]
    fn validate_value_panic() {
        assert!(validate_value("panic", "abort").is_ok());
        assert!(validate_value("panic", "unwind").is_ok());
        assert!(validate_value("panic", "immediate-abort").is_ok());
        assert!(validate_value("panic", "invalid").is_err());
    }

    #[test]
    fn validate_value_strip() {
        assert!(validate_value("strip", "none").is_ok());
        assert!(validate_value("strip", "debuginfo").is_ok());
        assert!(validate_value("strip", "symbols").is_ok());
        assert!(validate_value("strip", "invalid").is_err());
    }

    #[test]
    fn validate_value_profile() {
        assert!(validate_value("cargo.profile", "debug").is_ok());
        assert!(validate_value("cargo.profile", "release").is_ok());
        assert!(validate_value("cargo.profile", "invalid").is_err());
    }

    #[test]
    fn validate_value_lto() {
        assert!(validate_value("rustc.lto", "true").is_ok());
        assert!(validate_value("rustc.lto", "false").is_ok());
        assert!(validate_value("rustc.lto", "invalid").is_err());
    }

    #[test]
    fn validate_value_codegen_units() {
        assert!(validate_value("rustc.codegen_units", "1").is_ok());
        assert!(validate_value("rustc.codegen_units", "16").is_ok());
        assert!(validate_value("rustc.codegen_units", "abc").is_err());
    }

    #[test]
    fn validate_value_unconstrained() {
        assert!(validate_value("cargo.target_triple", "anything").is_ok());
        assert!(validate_value("cargo.target_dir", "anything").is_ok());
        assert!(validate_value("linker.args", "anything").is_ok());
    }

    #[test]
    fn set_toplevel_scalar() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "panic", "abort", FieldKind::Scalar).unwrap();
        assert_eq!(doc["panic"].as_str(), Some("abort"));
    }

    #[test]
    fn set_nested_scalar() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "rustc.lto", "true", FieldKind::Scalar).unwrap();
        assert_eq!(doc["rustc"]["lto"].as_bool(), Some(true));
    }

    #[test]
    fn set_nested_scalar_creates_table() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "cargo.profile", "release", FieldKind::Scalar).unwrap();
        assert_eq!(doc["cargo"]["profile"].as_str(), Some("release"));
    }

    #[test]
    fn set_array_bracket_syntax() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(
            &mut doc,
            "linker.args",
            r#"["-static", "-nostdlib"]"#,
            FieldKind::Array,
        )
        .unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn set_array_no_brackets_is_single_item() {
        // Without brackets, the entire string is one item (commas are literal)
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "rustc.build_std", "core,alloc", FieldKind::Array).unwrap();
        let arr = doc["rustc"]["build_std"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("core,alloc"));
    }

    #[test]
    fn set_array_single_item_no_brackets() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "linker.args", "-static", FieldKind::Array).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
    }

    #[test]
    fn set_codegen_units_as_integer() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "rustc.codegen_units", "1", FieldKind::Scalar).unwrap();
        assert_eq!(doc["rustc"]["codegen_units"].as_integer(), Some(1));
    }

    #[test]
    fn set_preserves_existing_content() {
        let input = "# My comment\npanic = \"unwind\"\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "strip", "symbols", FieldKind::Scalar).unwrap();
        let output = doc.to_string();
        assert!(output.contains("# My comment"));
        assert!(output.contains("strip = \"symbols\""));
    }

    #[test]
    fn unset_toplevel() {
        let input = "panic = \"abort\"\nstrip = \"symbols\"\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        unset_field(&mut doc, "panic").unwrap();
        assert!(doc.get("panic").is_none());
        assert!(doc.get("strip").is_some());
    }

    #[test]
    fn unset_nested() {
        let input = "[rustc]\nlto = true\nopt_level = \"3\"\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        unset_field(&mut doc, "rustc.lto").unwrap();
        assert!(doc["rustc"].get("lto").is_none());
        assert!(doc["rustc"].get("opt_level").is_some());
    }

    #[test]
    fn unset_removes_empty_table() {
        let input = "[rustc]\nlto = true\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        unset_field(&mut doc, "rustc.lto").unwrap();
        assert!(doc.get("rustc").is_none());
    }

    #[test]
    fn unset_nonexistent_is_ok() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        // Should not error
        unset_field(&mut doc, "panic").unwrap();
        unset_field(&mut doc, "rustc.lto").unwrap();
    }

    #[test]
    fn append_to_empty() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        append_field(&mut doc, "linker.args", "-static").unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
    }

    #[test]
    fn append_to_existing() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        append_field(&mut doc, "linker.args", "-nostdlib").unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn append_deduplicates() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        append_field(&mut doc, "linker.args", "-static").unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
    }

    #[test]
    fn append_multiple_bracket_syntax() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        append_field(&mut doc, "linker.args", r#"["-static", "-nostdlib"]"#).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn append_single_with_comma_is_literal() {
        // Without brackets, "-Wl,--gc-sections" is one item (not split on comma)
        let mut doc = "".parse::<DocumentMut>().unwrap();
        append_field(&mut doc, "linker.args", "-Wl,--gc-sections").unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-Wl,--gc-sections"));
    }

    #[test]
    fn remove_one_from_array() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_from_field(&mut doc, "linker.args", "-nostdlib").unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
    }

    #[test]
    fn remove_last_removes_field_and_table() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_from_field(&mut doc, "linker.args", "-static").unwrap();
        // Field removed, table removed (was empty)
        assert!(doc.get("linker").is_none());
    }

    #[test]
    fn remove_nonexistent_value_is_noop() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_from_field(&mut doc, "linker.args", "-nostdlib").unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
    }
}
