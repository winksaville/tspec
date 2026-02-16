//! toml_edit helpers for surgical tspec editing that preserves comments/formatting.

use anyhow::{Result, bail};
use toml_edit::{Array, DocumentMut, Item, Value};

/// Whether a field holds a scalar, an array, or a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Scalar,
    Array,
    Table,
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
    ("cargo.config_key_value", FieldKind::Table),
    ("rustc.opt_level", FieldKind::Scalar),
    ("rustc.lto", FieldKind::Scalar),
    ("rustc.codegen_units", FieldKind::Scalar),
    ("rustc.build_std", FieldKind::Array),
    ("rustc.flags", FieldKind::Array),
    ("linker.args", FieldKind::Array),
];

/// Validate that a key is in the registry and return its kind.
/// Also accepts table sub-keys like `cargo.config_key_value."profile.release.opt-level"`.
pub fn validate_key(key: &str) -> Result<FieldKind> {
    for &(k, kind) in FIELD_REGISTRY {
        if k == key {
            return Ok(kind);
        }
    }
    // Check if it's a table sub-key
    if parse_table_key(key).is_some() {
        return Ok(FieldKind::Table);
    }
    let valid_keys: Vec<&str> = FIELD_REGISTRY.iter().map(|(k, _)| *k).collect();
    bail!(
        "unknown key: {} (valid keys: {})",
        key,
        valid_keys.join(", ")
    )
}

/// Parse a key that may reference a sub-key within a Table field.
/// E.g., `cargo.config_key_value."profile.release.opt-level"` → Some(("cargo.config_key_value", "profile.release.opt-level"))
/// Also accepts unquoted sub-keys: `cargo.config_key_value.profile.release.opt-level` → same result.
/// Returns None if the key doesn't start with a known Table field prefix.
pub fn parse_table_key(key: &str) -> Option<(&str, &str)> {
    for &(prefix, kind) in FIELD_REGISTRY {
        if kind != FieldKind::Table {
            continue;
        }
        if let Some(rest) = key.strip_prefix(prefix)
            && let Some(sub_key) = rest.strip_prefix('.')
        {
            if sub_key.is_empty() {
                return None;
            }
            // Strip surrounding quotes if present
            let sub_key = sub_key
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(sub_key);
            if sub_key.is_empty() {
                return None;
            }
            return Some((prefix, sub_key));
        }
    }
    None
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

/// Set an array value into the document at the given key path.
fn set_array_in_doc(doc: &mut DocumentMut, key: &str, arr: Array) {
    let (table_name, field) = parse_key(key);
    match table_name {
        Some(table) => {
            ensure_table(doc, table);
            doc[table][field] = Item::Value(Value::Array(arr));
        }
        None => {
            doc[field] = Item::Value(Value::Array(arr));
        }
    }
}

/// Ensure a table exists in the document.
fn ensure_table(doc: &mut DocumentMut, table: &str) {
    if doc.get(table).is_none() {
        doc[table] = Item::Table(toml_edit::Table::new());
    }
}

// === Public API: takes &[String] from shell args, no string parsing ===

/// Set a field from string args. Scalars take `values[0]`, arrays take all values.
pub fn set_field(
    doc: &mut DocumentMut,
    key: &str,
    values: &[String],
    kind: FieldKind,
) -> Result<()> {
    let (table_name, field) = parse_key(key);

    match kind {
        FieldKind::Scalar => {
            if values.len() != 1 {
                bail!(
                    "scalar field '{}' requires exactly one value, got {}",
                    key,
                    values.len()
                );
            }
            let val = parse_scalar_value(key, &values[0]);
            match table_name {
                Some(table) => {
                    ensure_table(doc, table);
                    doc[table][field] = Item::Value(val);
                }
                None => {
                    doc[field] = Item::Value(val);
                }
            }
        }
        FieldKind::Array => {
            let mut arr = Array::new();
            for v in values {
                arr.push(v.as_str());
            }
            set_array_in_doc(doc, key, arr);
        }
        FieldKind::Table => {
            bail!(
                "use set_table_value() for table field '{}'; set_field() does not handle tables",
                key
            );
        }
    }

    Ok(())
}

/// Add items to an array field. Appends by default, or inserts at `index`.
/// Deduplicates on append; insert adds at position without dedup.
pub fn add_items(
    doc: &mut DocumentMut,
    key: &str,
    values: &[String],
    index: Option<usize>,
) -> Result<()> {
    let mut arr = get_existing_array(doc, key);

    match index {
        Some(idx) => {
            if idx > arr.len() {
                bail!(
                    "index {} out of bounds for array '{}' with {} elements",
                    idx,
                    key,
                    arr.len()
                );
            }
            // Insert at position (no dedup — user explicitly chose position)
            for (offset, v) in values.iter().enumerate() {
                arr.insert(idx + offset, v.as_str());
            }
        }
        None => {
            // Append with dedup
            let existing: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            for v in values {
                if !existing.contains(v) {
                    arr.push(v.as_str());
                }
            }
        }
    }

    set_array_in_doc(doc, key, arr);
    Ok(())
}

/// Remove items from an array field by value.
/// Keeps the field as an empty array if all items are removed.
pub fn remove_items_by_value(doc: &mut DocumentMut, key: &str, values: &[String]) -> Result<()> {
    let arr = get_existing_array(doc, key);

    let mut new_arr = Array::new();
    for item in arr.iter() {
        if let Some(s) = item.as_str()
            && !values.iter().any(|v| v == s)
        {
            new_arr.push(s);
        }
    }

    set_array_in_doc(doc, key, new_arr);
    Ok(())
}

/// Remove an item from an array field by index.
/// Keeps the field as an empty array if the last item is removed.
pub fn remove_item_by_index(doc: &mut DocumentMut, key: &str, index: usize) -> Result<()> {
    let mut arr = get_existing_array(doc, key);

    if index >= arr.len() {
        bail!(
            "index {} out of bounds for array '{}' with {} elements",
            index,
            key,
            arr.len()
        );
    }

    arr.remove(index);
    set_array_in_doc(doc, key, arr);
    Ok(())
}

/// Remove a field from a toml_edit document.
/// Does not remove the containing table, even if it becomes empty.
pub fn unset_field(doc: &mut DocumentMut, key: &str) -> Result<()> {
    let (table_name, field) = parse_key(key);

    match table_name {
        Some(table) => {
            if let Some(Item::Table(tbl)) = doc.get_mut(table) {
                tbl.remove(field);
            }
        }
        None => {
            doc.remove(field);
        }
    }

    Ok(())
}

/// Smart-parse a raw value string into a toml_edit Value for table entries.
/// Booleans → bool, integers → i64, everything else → string.
fn parse_smart_value(raw: &str) -> Value {
    match raw {
        "true" => Value::from(true),
        "false" => Value::from(false),
        _ => {
            if let Ok(n) = raw.parse::<i64>() {
                Value::from(n)
            } else {
                Value::from(raw)
            }
        }
    }
}

/// Set a value in a table field (e.g., `cargo.config_key_value`).
/// `table_path` is the dotted path to the table (e.g., "cargo.config_key_value").
/// `sub_key` is the key within that table (e.g., "profile.release.opt-level").
/// `raw_value` is the string value to set (auto-parsed to bool/int/string).
pub fn set_table_value(
    doc: &mut DocumentMut,
    table_path: &str,
    sub_key: &str,
    raw_value: &str,
) -> Result<()> {
    let (parent, table_name) = parse_key(table_path);
    let val = parse_smart_value(raw_value);

    match parent {
        Some(p) => {
            ensure_table(doc, p);
            // Ensure the nested table exists
            if doc[p].get(table_name).is_none() {
                doc[p][table_name] = Item::Table(toml_edit::Table::new());
            }
            doc[p][table_name][sub_key] = Item::Value(val);
        }
        None => {
            if doc.get(table_name).is_none() {
                doc[table_name] = Item::Table(toml_edit::Table::new());
            }
            doc[table_name][sub_key] = Item::Value(val);
        }
    }

    Ok(())
}

/// Remove a single key from a table field.
pub fn unset_table_value(doc: &mut DocumentMut, table_path: &str, sub_key: &str) -> Result<()> {
    let (parent, table_name) = parse_key(table_path);

    match parent {
        Some(p) => {
            if let Some(Item::Table(parent_tbl)) = doc.get_mut(p)
                && let Some(Item::Table(tbl)) = parent_tbl.get_mut(table_name)
            {
                tbl.remove(sub_key);
            }
        }
        None => {
            if let Some(Item::Table(tbl)) = doc.get_mut(table_name) {
                tbl.remove(sub_key);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to make a Vec<String> from string literals
    fn vs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

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

    // --- set_field tests ---

    #[test]
    fn set_toplevel_scalar() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "panic", &vs(&["abort"]), FieldKind::Scalar).unwrap();
        assert_eq!(doc["panic"].as_str(), Some("abort"));
    }

    #[test]
    fn set_nested_scalar() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "rustc.lto", &vs(&["true"]), FieldKind::Scalar).unwrap();
        assert_eq!(doc["rustc"]["lto"].as_bool(), Some(true));
    }

    #[test]
    fn set_nested_scalar_creates_table() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(
            &mut doc,
            "cargo.profile",
            &vs(&["release"]),
            FieldKind::Scalar,
        )
        .unwrap();
        assert_eq!(doc["cargo"]["profile"].as_str(), Some("release"));
    }

    #[test]
    fn set_array_multiple_values() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(
            &mut doc,
            "linker.args",
            &vs(&["-static", "-nostdlib"]),
            FieldKind::Array,
        )
        .unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn set_array_single_value() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "linker.args", &vs(&["-static"]), FieldKind::Array).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
    }

    #[test]
    fn set_codegen_units_as_integer() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_field(
            &mut doc,
            "rustc.codegen_units",
            &vs(&["1"]),
            FieldKind::Scalar,
        )
        .unwrap();
        assert_eq!(doc["rustc"]["codegen_units"].as_integer(), Some(1));
    }

    #[test]
    fn set_preserves_existing_content() {
        let input = "# My comment\npanic = \"unwind\"\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        set_field(&mut doc, "strip", &vs(&["symbols"]), FieldKind::Scalar).unwrap();
        let output = doc.to_string();
        assert!(output.contains("# My comment"));
        assert!(output.contains("strip = \"symbols\""));
    }

    #[test]
    fn set_scalar_rejects_multiple_values() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        let err = set_field(
            &mut doc,
            "panic",
            &vs(&["abort", "unwind"]),
            FieldKind::Scalar,
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("exactly one value"));
    }

    // --- unset_field tests ---

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
    fn unset_keeps_empty_table() {
        let input = "[rustc]\nlto = true\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        unset_field(&mut doc, "rustc.lto").unwrap();
        assert!(doc.get("rustc").is_some());
        assert!(doc["rustc"].get("lto").is_none());
    }

    #[test]
    fn unset_nonexistent_is_ok() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        unset_field(&mut doc, "panic").unwrap();
        unset_field(&mut doc, "rustc.lto").unwrap();
    }

    // --- add_items tests ---

    #[test]
    fn add_append_to_empty() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        add_items(&mut doc, "linker.args", &vs(&["-static"]), None).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
    }

    #[test]
    fn add_append_to_existing() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        add_items(&mut doc, "linker.args", &vs(&["-nostdlib"]), None).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn add_append_deduplicates() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        add_items(&mut doc, "linker.args", &vs(&["-static"]), None).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
    }

    #[test]
    fn add_append_multiple() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        add_items(
            &mut doc,
            "linker.args",
            &vs(&["-static", "-nostdlib"]),
            None,
        )
        .unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn add_insert_at_beginning() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        add_items(&mut doc, "linker.args", &vs(&["-nostartfiles"]), Some(0)).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-nostartfiles"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(2).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn add_insert_at_middle() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        add_items(
            &mut doc,
            "linker.args",
            &vs(&["-Wl,--gc-sections"]),
            Some(1),
        )
        .unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-Wl,--gc-sections"));
        assert_eq!(arr.get(2).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn add_insert_at_end() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        add_items(&mut doc, "linker.args", &vs(&["-nostdlib"]), Some(1)).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
        assert_eq!(arr.get(1).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn add_insert_out_of_bounds() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        let err = add_items(&mut doc, "linker.args", &vs(&["-static"]), Some(1));
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("out of bounds"));
    }

    #[test]
    fn add_insert_does_not_dedup() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        add_items(&mut doc, "linker.args", &vs(&["-static"]), Some(0)).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    // --- remove_items_by_value tests ---

    #[test]
    fn remove_one_by_value() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_items_by_value(&mut doc, "linker.args", &vs(&["-nostdlib"])).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
    }

    #[test]
    fn remove_multiple_by_value() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\", \"-Wl,--gc-sections\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_items_by_value(
            &mut doc,
            "linker.args",
            &vs(&["-static", "-Wl,--gc-sections"]),
        )
        .unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn remove_last_keeps_empty_array_and_table() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_items_by_value(&mut doc, "linker.args", &vs(&["-static"])).unwrap();
        assert!(doc.get("linker").is_some());
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn remove_nonexistent_value_is_noop() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_items_by_value(&mut doc, "linker.args", &vs(&["-nostdlib"])).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
    }

    // --- remove_item_by_index tests ---

    #[test]
    fn remove_by_index_first() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_item_by_index(&mut doc, "linker.args", 0).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-nostdlib"));
    }

    #[test]
    fn remove_by_index_last() {
        let input = "[linker]\nargs = [\"-static\", \"-nostdlib\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_item_by_index(&mut doc, "linker.args", 1).unwrap();
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().as_str(), Some("-static"));
    }

    #[test]
    fn remove_by_index_last_item_keeps_empty_array() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        remove_item_by_index(&mut doc, "linker.args", 0).unwrap();
        assert!(doc.get("linker").is_some());
        let arr = doc["linker"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn remove_by_index_out_of_bounds() {
        let input = "[linker]\nargs = [\"-static\"]\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        let err = remove_item_by_index(&mut doc, "linker.args", 5);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("out of bounds"));
    }

    // --- Table field tests ---

    #[test]
    fn validate_key_table() {
        assert_eq!(
            validate_key("cargo.config_key_value").unwrap(),
            FieldKind::Table
        );
    }

    #[test]
    fn validate_key_table_subkey() {
        assert_eq!(
            validate_key("cargo.config_key_value.\"profile.release.opt-level\"").unwrap(),
            FieldKind::Table
        );
    }

    #[test]
    fn validate_key_table_subkey_unquoted() {
        assert_eq!(
            validate_key("cargo.config_key_value.profile.release.opt-level").unwrap(),
            FieldKind::Table
        );
    }

    #[test]
    fn parse_table_key_quoted() {
        let result = parse_table_key("cargo.config_key_value.\"profile.release.opt-level\"");
        assert_eq!(
            result,
            Some(("cargo.config_key_value", "profile.release.opt-level"))
        );
    }

    #[test]
    fn parse_table_key_unquoted() {
        let result = parse_table_key("cargo.config_key_value.profile.release.opt-level");
        assert_eq!(
            result,
            Some(("cargo.config_key_value", "profile.release.opt-level"))
        );
    }

    #[test]
    fn parse_table_key_not_table() {
        assert!(parse_table_key("rustc.lto").is_none());
        assert!(parse_table_key("panic").is_none());
    }

    #[test]
    fn parse_table_key_bare_table() {
        assert!(parse_table_key("cargo.config_key_value").is_none());
    }

    #[test]
    fn set_table_value_string() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_table_value(
            &mut doc,
            "cargo.config_key_value",
            "profile.release.opt-level",
            "s",
        )
        .unwrap();
        assert_eq!(
            doc["cargo"]["config_key_value"]["profile.release.opt-level"].as_str(),
            Some("s")
        );
    }

    #[test]
    fn set_table_value_bool() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_table_value(
            &mut doc,
            "cargo.config_key_value",
            "profile.release.lto",
            "true",
        )
        .unwrap();
        assert_eq!(
            doc["cargo"]["config_key_value"]["profile.release.lto"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn set_table_value_integer() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        set_table_value(
            &mut doc,
            "cargo.config_key_value",
            "profile.release.codegen-units",
            "1",
        )
        .unwrap();
        assert_eq!(
            doc["cargo"]["config_key_value"]["profile.release.codegen-units"].as_integer(),
            Some(1)
        );
    }

    #[test]
    fn set_table_value_overwrites() {
        let input = "[cargo.config_key_value]\n\"profile.release.opt-level\" = \"s\"\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        set_table_value(
            &mut doc,
            "cargo.config_key_value",
            "profile.release.opt-level",
            "z",
        )
        .unwrap();
        assert_eq!(
            doc["cargo"]["config_key_value"]["profile.release.opt-level"].as_str(),
            Some("z")
        );
    }

    #[test]
    fn unset_table_value_removes_key() {
        let input = "[cargo.config_key_value]\n\"profile.release.opt-level\" = \"s\"\n\"profile.release.lto\" = true\n";
        let mut doc = input.parse::<DocumentMut>().unwrap();
        unset_table_value(
            &mut doc,
            "cargo.config_key_value",
            "profile.release.opt-level",
        )
        .unwrap();
        assert!(
            doc["cargo"]["config_key_value"]
                .get("profile.release.opt-level")
                .is_none()
        );
        assert_eq!(
            doc["cargo"]["config_key_value"]["profile.release.lto"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn unset_table_value_nonexistent_is_ok() {
        let mut doc = "".parse::<DocumentMut>().unwrap();
        unset_table_value(&mut doc, "cargo.config_key_value", "nonexistent").unwrap();
    }
}
