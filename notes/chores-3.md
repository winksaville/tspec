# Chores-3

Continued maintenance tasks, following on from chores-2.

## 20260209 - ts set/unset/backup/restore improvements using toml_edit

### Context

`ts set` currently only supports ~10 hardcoded scalar keys and silently ignores all array/nested fields (`rustc.build_std`, `rustc.flags`, `linker.args`, `cargo.unstable`, `cargo.target_json`, `linker.version_script`). Users get `Error: unknown key` for documented fields. Additionally, `ts backup`, `ts restore`, and `ts new -f` round-trip through serde serialization, which destroys user comments and formatting.

### Three write strategies

| Operation | Strategy | Why |
|---|---|---|
| load, hash, build | serde (`toml`) | Need typed `Spec` struct for build logic |
| set, unset | `toml_edit` (`DocumentMut`) | Surgical edit preserving comments/formatting |
| backup, restore, new -f | raw `fs::copy` | Exact byte-for-byte preservation |

### Step 1: Add `toml_edit` dependency

**File:** `Cargo.toml`

Add `toml_edit = "0.22"` to `[dependencies]`. Already a transitive dep via `toml 0.8`, so no new download.

### Step 2: Create `src/ts_cmd/edit.rs` - toml_edit helper module

New module with:

- **`FIELD_REGISTRY`** - static list of all valid field paths with their types:
  ```
  ("panic", Scalar)           ("strip", Scalar)
  ("cargo.profile", Scalar)   ("cargo.target_triple", Scalar)
  ("cargo.target_json", Scalar) ("cargo.target_dir", Scalar)
  ("cargo.unstable", Array)   ("rustc.opt_level", Scalar)
  ("rustc.panic", Scalar)     ("rustc.lto", Scalar)
  ("rustc.codegen_units", Scalar) ("rustc.build_std", Array)
  ("rustc.flags", Array)      ("linker.args", Array)
  ```
  (Skip `linker.version_script` for now - it's a nested table with `global` array + `local` string, more complex.)

- **`validate_key(key) -> Result<FieldKind>`** - checks key against registry, returns `Scalar` or `Array`

- **`set_field(doc, key, value) -> Result<()>`** - for scalars: parse key path -> ensure table exists -> set `doc["section"]["field"] = value`. Value parsing: booleans as `toml_edit::value(true/false)`, integers as `toml_edit::value(i64)`, everything else as string. For arrays: parse `[a,b,c]` or `"a","b"` syntax -> replace the array.

- **`unset_field(doc, key) -> Result<()>`** - parse key path -> remove from table. Does not remove the containing table, even if it becomes empty.

- **`validate_value(key, value) -> Result<()>`** - reuse existing parse functions (`parse_panic_mode`, `parse_strip_mode`, etc.) for enum keys. Accept anything for string/array keys.

### Step 3: Rewrite `src/ts_cmd/set.rs` to use toml_edit

Replace serde round-trip with:
1. Read file as raw string (or empty string if file doesn't exist)
2. Parse into `DocumentMut`
3. Validate key via `edit::validate_key()`
4. Validate value via `edit::validate_value()` (for enum fields)
5. Call `edit::set_field(doc, key, value)`
6. Write `doc.to_string()` back to file

Remove `apply_value()` and all parse helper functions from set.rs (move validation to edit.rs).

Update tests to test the new toml_edit-based flow. Add tests for previously-missing keys (`rustc.build_std`, `linker.args`, `cargo.unstable`, etc.).

### Step 4: Create `src/ts_cmd/unset.rs` - new command

New file:
1. Resolve package dir and tspec path (same pattern as set.rs)
2. Read file as raw string
3. Parse into `DocumentMut`
4. Validate key via `edit::validate_key()`
5. Call `edit::unset_field(doc, key)`
6. Write back

### Step 5: Wire up `ts unset` in CLI

**Files:** `src/cmd/ts.rs`, `src/ts_cmd/mod.rs`

- Add `Unset` variant to `TsCommands` enum (same args as `Set` minus the value):
  ```
  Unset { key, package, tspec }
  ```
- Add `unset_value` to `ts_cmd/mod.rs` re-exports
- Add match arm in `TsCmd::execute()`

### Step 6: Convert `ts backup` to raw file copy

**Files:** `src/ts_cmd/backup.rs`, `src/tspec.rs`

Current flow: `load_spec` -> `save_spec_snapshot` (serde serialize) - destroys comments.

New flow:
1. Find the spec path (same as now)
2. `load_spec` **only for hashing** (to compute the hash for the backup filename)
3. Compute next sequence number (same logic as `save_spec_snapshot`)
4. `fs::copy(source_path, backup_path)` - exact byte-for-byte copy

Extract the sequence-number logic from `save_spec_snapshot` into a helper `next_snapshot_seq(name, dir)` in `tspec.rs` so both old code and new backup can use it.

### Step 7: Convert `ts restore` to raw file copy

**File:** `src/ts_cmd/restore.rs`

Current flow: `load_spec(backup)` -> `save_spec(target)` - destroys comments.

New flow:
1. Find backup path (same as now)
2. Parse base name (same as now)
3. `fs::copy(backup_path, target_path)` - exact copy

### Step 8: Convert `ts new -f` to raw file copy

**File:** `src/ts_cmd/new.rs`

Current flow: `load_spec(source)` -> `save_spec(target)` - destroys comments.

New flow when `--from` is provided:
1. Resolve source spec path (same as now)
2. `fs::copy(source_path, output_path)` - exact copy

When `--from` is NOT provided, keep using serde `save_spec(&Spec::default(), ...)` since there's no source to copy.

### Step 9 (nice-to-have): Array append/remove syntax in `ts set`

Extend `ts set` to support:
- `tspec ts set linker.args+=-Wl,--gc-sections` (append to array)
- `tspec ts set linker.args-=-Wl,--gc-sections` (remove from array)
- `tspec ts set linker.args=["-static","-nostdlib"]` (replace entire array, from step 3)

This builds on the `edit::set_field` infrastructure. Can be deferred if the basic set/unset is solid.

### Step 10: Update todo.md

Mark completed items, remove resolved entries.

### Files to modify

| File | Change |
|---|---|
| `Cargo.toml` | Add `toml_edit` dep |
| `src/ts_cmd/edit.rs` | **NEW** - toml_edit helpers |
| `src/ts_cmd/set.rs` | Rewrite to use toml_edit |
| `src/ts_cmd/unset.rs` | **NEW** - unset command |
| `src/ts_cmd/mod.rs` | Add unset + edit modules |
| `src/cmd/ts.rs` | Add Unset variant |
| `src/ts_cmd/backup.rs` | Use fs::copy |
| `src/ts_cmd/restore.rs` | Use fs::copy |
| `src/ts_cmd/new.rs` | Use fs::copy for --from |
| `src/tspec.rs` | Extract `next_snapshot_seq()` helper |
| `notes/todo.md` | Update status |

### Verification

```bash
tspec test -p tspec          # All existing + new tests pass
tspec clippy                 # No warnings
tspec fmt --check            # Formatted

# Manual smoke tests:
tspec ts set rustc.build_std='["core","alloc"]'    # Previously: "unknown key"
tspec ts set linker.args='["-static"]'             # Previously: "unknown key"
tspec ts unset rustc.lto                           # New command
tspec ts backup && diff original backup            # Byte-identical
tspec ts restore -t name-001-hash                  # Preserves comments
```

## 20260210 - Shell-hostile characters in values and array syntax

### Problem

Several tspec features used characters that conflict with bash:

1. **`<name>`/`<hash>` placeholders in `cargo.target_dir`** — `<` and `>` are shell
   redirection operators, so `tspec ts set cargo.target_dir=<name>-xyz` triggers
   `bash: name: No such file or directory`. Users must quote the whole argument.

2. **Bracket array syntax with unquoted values** — `tspec ts set linker.args-=[za]`
   fails because `parse_array_value` tries to parse `[za]` as a TOML inline array,
   and bare `za` is not valid TOML (must be `["za"]`). Meanwhile `linker.args+=za`
   (no brackets) works fine since the bare path treats it as a single string.

3. **Shell quote stripping compounds the bracket problem** — even
   `tspec ts set linker.args-=["za"]` fails because bash strips the double quotes,
   passing `[za]` to tspec. Only single-quoting the entire argument works:
   `tspec ts set 'linker.args-=["za"]'`.

### Fix 1: `<>` → `{}` placeholders (v0.9.8)

Replaced `<name>`/`<hash>` with `{name}`/`{hash}` in `expand_target_dir()`. Curly
braces are shell-inert — bash brace expansion only triggers with commas (`{a,b}`) or
ranges (`{1..3}`) inside, so `{name}` passes through untouched.

**Files changed:** `src/tspec.rs`, `src/types.rs`, `src/ts_cmd/set.rs` (test),
`README.md`

### Fix 2: Bare-string fallback for bracket array syntax (v0.9.8)

Changed `parse_array_value()` to try TOML parsing first, then fall back to
comma-splitting bare strings when TOML fails. This means:

| Input | Parse path | Result |
|---|---|---|
| `za` | bare (no brackets) | single item `"za"` |
| `[za]` | TOML fails → bare fallback | single item `"za"` |
| `[-static,-nostdlib]` | TOML fails → bare fallback | two items |
| `[-static, -nostdlib]` | TOML fails → bare fallback | two items (spaces trimmed) |
| `["-Wl,--gc-sections", "-static"]` | TOML succeeds | two items (embedded comma preserved) |

The TOML-first path is preserved so that properly quoted strings with embedded commas
still work — `"-Wl,--gc-sections"` contains a comma that must not be split on.

**Files changed:** `src/ts_cmd/edit.rs` (7 new tests)

### Bash glob safety of square brackets

Square brackets are bash glob characters — `[fx]` matches a single-character filename
from the set {f, x}. Initial concern was that `linker.args+=[-static,-nostdlib]` could
be glob-expanded. Investigation showed this is safe because **bash globs the entire
token**, not just the bracket portion. The token `linker.args+=[-static,-nostdlib]`
would only match a file literally named `linker.args+=-` (or `s`, `t`, etc.), which
will never exist.

Standalone brackets DO expand:

```bash
$ echo [fx]          # no f or x file
[fx]
$ touch f
$ echo [fx]          # f exists
f
$ touch x
$ echo [fx]          # both exist
f x
$ rm f
$ echo [fx]          # only x
x
$ rm x
$ echo [fx]          # neither exists, passes through
[fx]
```

But with a prefix, the whole token fails to match any file:

```bash
$ touch f x
$ echo [fx]          # standalone: expands
f x
$ echo a=[fx]        # prefixed: no file matches "a=f" or "a=x"
a=[fx]
```

**Conclusion:** Square brackets are safe unquoted in `tspec ts set key+=[values]`
syntax. No quoting needed for normal use.

## 20260211 - Orthogonal `ts set`/`add`/`remove` with separate key and value args ✓

### Problem

`ts set` takes the entire assignment as a single positional argument (`key=value`),
which forces users to fight the shell:

```bash
# Scalar — fine
tspec ts set rustc.lto=true

# Array — bracket/quote gymnastics
tspec ts set 'linker.args=["-static","-nostdlib"]'

# Append — still awkward
tspec ts set 'linker.args+=-Wl,--gc-sections'
```

The root cause is two-fold:

1. **Key and value crammed into one shell token** — any special characters in
   the value (brackets, quotes, spaces) force quoting the entire `key=value`.
2. **Overloaded operators (`=`, `+=`, `-=`) encoded in a string** — three
   fundamentally different operations (replace, append, remove) masquerade as
   modes of one command via string parsing.

### Design: Orthogonal verbs with separate args

Split into four commands on two axes:

| Axis | Command | Purpose |
|---|---|---|
| Field-level | `ts set` | Set scalar or replace entire array |
| Field-level | `ts unset` | Remove entire field (already exists) |
| Item-level | `ts add` | Add items to array (append or insert) |
| Item-level | `ts remove` | Remove items from array (by value or index) |

Key and value are separate clap positional arguments. Array values are variadic
(each element is its own shell arg — no brackets, no inner quotes).

### Hyphen-prefixed values and `allow_hyphen_values`

Values like `-static` and `-nostdlib` start with `-`, which clap normally tries
to parse as flags. The fix is `#[arg(allow_hyphen_values = true)]` on the
`value` field — clap then treats unknown hyphen-prefixed tokens as positional
values. This means **no `--` needed** in normal use:

```bash
tspec ts set linker.args -static -nostdlib      # just works
tspec ts add linker.args -Wl,--gc-sections      # just works
```

The only edge case is a value that exactly matches a defined short flag (`-p`,
`-t`, `-i`). In practice no real linker/rustc flag is just `-p` or `-t`, but
`--` remains available as an escape hatch for that theoretical collision.

### Syntax

```bash
# === Field-level (set/unset) ===

# Scalars — just two words
tspec ts set rustc.lto true
tspec ts set cargo.profile release
tspec ts set panic abort

# Replace entire array — each element is a separate arg
tspec ts set linker.args -static -nostdlib
tspec ts set rustc.build_std core alloc

# Remove entire field
tspec ts unset linker.args

# === Item-level (add/remove) ===

# Append (default add behavior)
tspec ts add linker.args -Wl,--gc-sections
tspec ts add rustc.flags -Cforce-frame-pointers=yes -Clink-dead-code=no

# Insert at position
tspec ts add -i 0 linker.args -nostdlib

# Remove by value
tspec ts remove linker.args -static
tspec ts remove linker.args -static -nostdlib

# Remove by index
tspec ts remove -i 2 linker.args
```

### The six array operations

| # | Operation | Command | Example |
|---|---|---|---|
| 1 | Replace all | `ts set` | `ts set linker.args -static -nostdlib` |
| 2 | Clear field | `ts unset` | `ts unset linker.args` |
| 3 | Append items | `ts add` | `ts add linker.args -Wl,--gc-sections` |
| 4 | Insert at position | `ts add -i N` | `ts add -i 0 linker.args -nostdlib` |
| 5 | Remove by value | `ts remove` | `ts remove linker.args -static` |
| 6 | Remove by index | `ts remove -i N` | `ts remove -i 2 linker.args` |

### Clap definitions

```rust
/// Set a field (scalar value or replace entire array)
Set {
    /// Field key (e.g., "rustc.lto", "linker.args")
    key: String,
    /// Value(s). For scalars, one value. For arrays, each arg is an element.
    #[arg(required = true, allow_hyphen_values = true)]
    value: Vec<String>,
    #[arg(short = 'p', long = "package")]
    package: Option<String>,
    #[arg(short = 't', long = "tspec")]
    tspec: Option<String>,
}

/// Add items to an array field (append by default, or insert at position)
Add {
    /// Field key (must be an array field)
    key: String,
    /// Items to add
    #[arg(required = true, allow_hyphen_values = true)]
    value: Vec<String>,
    /// Insert at this index instead of appending
    #[arg(short = 'i', long = "index")]
    index: Option<usize>,
    #[arg(short = 'p', long = "package")]
    package: Option<String>,
    #[arg(short = 't', long = "tspec")]
    tspec: Option<String>,
}

/// Remove items from an array field (by value or by index)
Remove {
    /// Field key (must be an array field)
    key: String,
    /// Items to remove by value (not used with --index)
    #[arg(allow_hyphen_values = true)]
    value: Vec<String>,
    /// Remove item at this index instead of by value
    #[arg(short = 'i', long = "index")]
    index: Option<usize>,
    #[arg(short = 'p', long = "package")]
    package: Option<String>,
    #[arg(short = 't', long = "tspec")]
    tspec: Option<String>,
}
```

### Validation rules

- `ts set` on a scalar field: exactly one value required
- `ts set` on an array field: one or more values (replaces entire array)
- `ts add`: key must be an array field (error on scalar)
- `ts add -i N`: index must be ≤ current array length
- `ts remove` without `--index`: at least one value required
- `ts remove -i N` with `--index`: no values expected (index is the selector)
- `ts remove -i N`: index must be < current array length

### Implementation steps

#### Step 1: Add `add_items` and `remove_items` to `edit.rs`

New functions in `src/ts_cmd/edit.rs`:

- **`add_items(doc, key, values: &[String], index: Option<usize>)`** — like
  `append_field` but takes a `Vec<String>` directly (no string parsing), with
  optional insert-at-index. Deduplicates on append; for insert, adds at position
  without dedup (user explicitly chose the position).

- **`remove_items_by_value(doc, key, values: &[String])`** — like
  `remove_from_field` but takes `Vec<String>` directly. Keeps the field as an
  empty array if all items are removed (tables are never auto-removed).

- **`remove_item_by_index(doc, key, index: usize)`** — removes the item at
  the given index. Keeps the field as an empty array if the last item is removed.

- **`set_field_from_strings(doc, key, values: &[String], kind)`** — for `ts set`.
  Scalars: `values[0]`. Arrays: builds array from all values. Replaces
  `set_field(doc, key, value_str, kind)` which parsed from a single string.

Keep `parse_array_value()` and `parse_scalar_value()` as internal helpers but
the public API takes `&[String]` — the shell already did the splitting.

#### Step 2: Add `add_value` and `remove_value` to `ts_cmd/`

**New file:** `src/ts_cmd/add.rs`

```rust
pub fn add_value(
    project_root: &Path,
    package: Option<&str>,
    key: &str,
    values: &[String],
    index: Option<usize>,
    tspec: Option<&str>,
) -> Result<()>
```

- Validate key is an array field (error if scalar)
- Read, parse DocumentMut, call `edit::add_items()`, write back

**New file:** `src/ts_cmd/remove.rs`

```rust
pub fn remove_value(
    project_root: &Path,
    package: Option<&str>,
    key: &str,
    values: &[String],
    index: Option<usize>,
    tspec: Option<&str>,
) -> Result<()>
```

- Validate key is an array field (error if scalar)
- If `index` is Some: call `edit::remove_item_by_index()`
- If `index` is None and values non-empty: call `edit::remove_items_by_value()`
- If neither: error

#### Step 3: Rewrite `set_value` to accept `&[String]`

**File:** `src/ts_cmd/set.rs`

- Change signature: `value: &str` → `values: &[String]`
- For scalar fields: validate `values.len() == 1`, then use `values[0]`
- For array fields: pass all values to `edit::set_field_from_strings()`
- Remove `parse_assignment()` from `src/cmd/ts.rs`

#### Step 4: Wire up `Add` and `Remove` in CLI

**File:** `src/cmd/ts.rs`

- Add `Add` and `Remove` variants to `TsCommands`
- Add match arms in `TsCmd::execute()`
- Remove `parse_assignment()` (no longer needed)
- Update `Set` variant: `assignment: String` → `key: String` + `value: Vec<String>`

**File:** `src/ts_cmd/mod.rs`

- Add `mod add; mod remove;`
- Add `pub use add::add_value; pub use remove::remove_value;`
- Remove `pub use edit::SetOp;` (no longer needed in public API)

#### Step 5: Clean up dead code

- Remove `SetOp` enum from `edit.rs` (or make it `pub(crate)`)
- Remove `append_field()`, `remove_from_field()`, `parse_array_value()`
  if no longer called (the new functions take `&[String]` instead of
  parsing bracket syntax)
- Remove `parse_assignment()` from `cmd/ts.rs`

#### Step 6: Update tests

- Rewrite `set.rs` tests to use the new `values: &[String]` signature
- Add `add.rs` tests: append, insert-at-index, dedup, scalar-rejection
- Add `remove.rs` tests: by-value, by-index, empty-keeps-array,
  scalar-rejection, out-of-bounds index error
- Add `edit.rs` unit tests for the new functions
- Remove tests for old bracket-syntax parsing (dead code)

#### Step 7: Update README

- Update `ts set` examples to new syntax
- Document `ts add` and `ts remove` commands

### Files to modify

| File | Change |
|---|---|
| `src/ts_cmd/edit.rs` | Add `add_items`, `remove_items_by_value`, `remove_item_by_index`, `set_field_from_strings`; clean up old API |
| `src/ts_cmd/set.rs` | Rewrite to accept `&[String]` |
| `src/ts_cmd/add.rs` | **NEW** — `add_value` |
| `src/ts_cmd/remove.rs` | **NEW** — `remove_value` |
| `src/ts_cmd/mod.rs` | Register new modules, update exports |
| `src/cmd/ts.rs` | Add `Add`/`Remove` variants, rewrite `Set`, remove `parse_assignment()` |
| `README.md` | Update syntax examples |

### Backward compatibility

The old single-arg `key=value` / `key+=value` / `key-=value` syntax is removed.
This is a breaking change, but the old syntax is the problem — keeping it as a
fallback perpetuates the quoting issues.

### Verification

```bash
tspec test -p tspec
tspec clippy
tspec fmt --check

# Manual smoke tests:
tspec ts set rustc.lto true
tspec ts set cargo.profile release
tspec ts set linker.args -static -nostdlib
tspec ts add linker.args -Wl,--gc-sections
tspec ts add -i 0 linker.args -nostartfiles
tspec ts remove linker.args -static
tspec ts remove -i 0 linker.args
tspec ts unset linker.args
```
