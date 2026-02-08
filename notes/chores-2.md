# Chores-2

Continued maintenance tasks, following on from chores-1.

## 20260206 - Ts Execute Conversion

### Convert `Commands::Ts` to use Execute trait

The `Ts` variant in `Commands` is the only command that doesn't use the `Execute` trait. All other commands (Build, Run, Test, Clean, Clippy, Fmt, Compare, Version, Install) follow the pattern where a command struct implements `Execute` and main.rs calls `cmd.execute(&project_root)`. The `Ts` subcommands instead destructure fields in main.rs and call free functions from `ts_cmd::*`.

**Current state in main.rs:**
```rust
Commands::Ts { command } => match command {
    TsCommands::List { package, all } => {
        ts_cmd::list_tspecs(package.as_deref(), all)?;
    }
    TsCommands::Show { package, all, tspec } => {
        ts_cmd::show_tspec(package.as_deref(), all, tspec.as_deref())?;
    }
    TsCommands::Hash { package, all, tspec } => {
        ts_cmd::hash_tspec(package.as_deref(), all, tspec.as_deref())?;
    }
    TsCommands::New { name, package, from } => {
        ts_cmd::new_tspec(package.as_deref(), &name, from.as_deref())?;
    }
    TsCommands::Set { assignment, package, tspec } => {
        let (key, value) = assignment.split_once('=')...;
        ts_cmd::set_value(package.as_deref(), key, value, tspec.as_deref())?;
    }
},
```

**Goal state in main.rs:**
```rust
Commands::Ts(cmd) => {
    cmd.execute(&find_project_root()?)?;
}
```

**Implementation steps:**

1. **Create `src/cmd/ts.rs`** with a `TsCmd` struct that wraps `TsCommands`:
   ```rust
   #[derive(Args)]
   pub struct TsCmd {
       #[command(subcommand)]
       command: TsCommands,
   }

   impl Execute for TsCmd {
       fn execute(&self, project_root: &Path) -> Result<ExitCode> {
           match &self.command {
               TsCommands::List { package, all } => {
                   ts_cmd::list_tspecs(package.as_deref(), *all)?;
               }
               // ... other variants
           }
           Ok(ExitCode::SUCCESS)
       }
   }
   ```

2. **Move `TsCommands` enum** from `cli.rs` to `src/cmd/ts.rs` (or keep in cli.rs and import — either works, but co-locating with the Execute impl is cleaner).

3. **Update `cli.rs`** to use `Ts(TsCmd)` tuple variant instead of `Ts { command: TsCommands }` struct variant:
   ```rust
   // Before
   Ts {
       #[command(subcommand)]
       command: TsCommands,
   },
   // After
   Ts(TsCmd),
   ```

4. **Update `src/cmd/mod.rs`** to add `mod ts;` and `pub use ts::TsCmd;`.

5. **Update main.rs** to replace the 30-line `Ts` match arm with the one-liner.

6. **Consider whether `ts_cmd/` functions need `project_root`** — currently they call `find_project_root()` internally or use `current_dir()`. The Execute trait passes `project_root`, so the ts_cmd functions could be updated to accept it instead of finding it themselves. This is optional but would make them consistent.

**Files affected:**
- `src/cmd/ts.rs` (new)
- `src/cmd/mod.rs` (add module + re-export)
- `src/cli.rs` (change `Ts` variant, possibly move `TsCommands`)
- `src/main.rs` (simplify match arm)
- `src/ts_cmd/*.rs` (optional: accept `project_root` parameter)

**Status:** Done

## 20260206 - Add `cargo.target_dir` spec field

### The Problem

When two specs share the same target triple, builds overwrite each other in `target/{triple}/{profile}/`. This makes `tspec compare` unreliable and prevents side-by-side builds.

### The Design

Add a `target_dir` field to the `[cargo]` spec section with template placeholder support:

```toml
[cargo]
target_dir = "<name>"           # spec filename sans .ts.toml
target_dir = "<hash>"           # 8-char content hash
target_dir = "<name>-<hash>"    # combined
target_dir = ""                 # empty = no subdir (backward compat)
```

Path structure uses cargo's native `--target-dir`: `target/{target_dir}/{triple}/{profile}/{binary}`

Default when field absent: empty (backward compatible, no subdirectory).

### The Plan

The `target_dir` field threads through the build pipeline: types → spec loading → path resolution → cargo command → binary scanning. The double `load_spec` call in `build_crate` gets consolidated along the way.

1. **Add `target_dir` field to `CargoConfig`** (`src/types.rs`)
   - Add `pub target_dir: Option<String>` to `CargoConfig`

2. **Add helpers in `src/tspec.rs`**
   - `spec_name_from_path(path: &Path) -> String` — strips `.ts.toml` suffix from filename
   - `expand_target_dir(spec: &Spec, spec_name: &str) -> Result<Option<String>>` — expands `<name>` and `<hash>` placeholders; returns `None` if field is absent or empty

3. **Update `get_binary_path`** (`src/find_paths.rs`)
   - Add `expanded_target_dir: Option<&str>` parameter
   - When `Some(td)`, base path becomes `workspace.join("target").join(td)` instead of `workspace.join("target")`
   - Update existing test call sites to pass `None`

4. **Update `apply_spec_to_command`** (`src/cargo_build.rs`)
   - Add `expanded_target_dir: Option<&str>` parameter
   - When `Some(td)`, insert `--target-dir target/{td}` into cargo command
   - Fix version script path to use expanded target dir

5. **Refactor `build_crate`** (`src/cargo_build.rs`)
   - Consolidate the double `load_spec` call into one
   - Compute `spec_name` and `expanded_td` after loading
   - Pass `expanded_td` to both `get_binary_path` and `apply_spec_to_command`
   - Add `target_base: PathBuf` to `BuildResult`

6. **Update `test_crate`** (`src/testing.rs`)
   - Compute `expanded_td` from loaded spec and pass to `apply_spec_to_command`

7. **Fix test binary scanning** (`src/all.rs`)
   - Capture `BuildResult` from `build_crate` (currently only error case captured)
   - Use `build_result.target_base.join(profile)` instead of hardcoded `workspace.root.join("target").join(profile)`

8. **Add `cargo.target_dir` to `tspec ts set`** (`src/ts_cmd/set.rs`)
   - Add match arm for `"cargo.target_dir"` in `apply_value`

9. **Tests**
   - Unit tests for `expand_target_dir` (None, empty, literal, `<name>`, `<hash>`, combined)
   - Unit tests for `spec_name_from_path`
   - Unit test for `get_binary_path` with target_dir
   - Unit test for `apply_value` with `cargo.target_dir`
   - Existing tests: append `None` to `get_binary_path` calls (mechanical)

**Status:** Done

## 20260207 - In-place `set`, add `backup` and `restore` subcommands

### The Problem

`tspec ts set` creates a new snapshot file (`name-NNN-hash.ts.toml`) on every edit.
Chaining edits compounds the names: `t1-001-xxx-001-yyy.ts.toml`. The file you work
with should keep its name; backup/restore should be explicit user actions.

### The Design

1. **`tspec ts set`** modifies the file in place (no snapshot creation)
2. **`tspec ts backup -t t1.ts.toml`** creates `t1-001-<hash>.ts.toml`
3. **`tspec ts restore -t t1-001-<hash>.ts.toml`** copies it back to `t1.ts.toml`
4. Backups are valid specs — `tspec build -t t1-001-<hash>.ts.toml` works directly

### The Plan

#### 1. Modify `tspec ts set` to save in place

**File:** `src/ts_cmd/set.rs`

- Change `set_value()` to save back to the original file path instead of calling `save_spec_snapshot()`
- If the file existed (found via `find_tspec()`), save to that path
- If no file existed, construct path from `-t` arg (or default `tspec`) + `TSPEC_SUFFIX` in package dir
- Use existing `save_spec()` instead of `save_spec_snapshot()`
- Remove `save_spec_snapshot` import

**File:** `src/cmd/ts.rs`
- Update `Set` variant doc comment from "creates versioned copy" to "modifies in place"

#### 2. Add `tspec ts backup` subcommand

**New file:** `src/ts_cmd/backup.rs`

```rust
pub fn backup_tspec(project_root, package, tspec) -> Result<()>
```

- Locate the spec via `find_tspec()`; error if not found
- Extract base name via `spec_name_from_path()`
- Load the spec, call existing `save_spec_snapshot()` to create `{name}-NNN-{hash}.ts.toml`
- Print the backup filename

**File:** `src/cmd/ts.rs` — add `Backup` variant with `-p` and `-t` args, dispatch
**File:** `src/ts_cmd/mod.rs` — add `mod backup;` and `pub use backup::backup_tspec;`

#### 3. Add `tspec ts restore` subcommand

**New file:** `src/ts_cmd/restore.rs`

```rust
pub fn restore_tspec(project_root, package, tspec) -> Result<()>
```

- Locate the backup file via `find_tspec()` (`-t` required)
- Parse the filename to extract the base name by stripping trailing `-NNN-HHHHHHHH`
  (3-digit seq + 8-char hex hash) from the stem before `.ts.toml`
- Construct target path: `{base_name}.ts.toml` in the same directory
- Copy content to the base file using `load_spec()` + `save_spec()`
- Print what was restored

**File:** `src/cmd/ts.rs` — add `Restore` variant with `-p` and `-t` (required) args, dispatch
**File:** `src/ts_cmd/mod.rs` — add `mod restore;` and `pub use restore::restore_tspec;`

### Files modified
- `src/ts_cmd/set.rs` — save in place instead of snapshot
- `src/cmd/ts.rs` — add Backup/Restore variants, update Set doc
- `src/ts_cmd/mod.rs` — register new modules

### Files created
- `src/ts_cmd/backup.rs` — backup logic
- `src/ts_cmd/restore.rs` — restore logic (with name-suffix parsing)

### Verification
```bash
tspec test -p tspec
tspec clippy
tspec fmt --check

# Manual smoke test:
tspec ts new t2
tspec ts set strip=symbols -t t2.ts.toml      # modifies t2.ts.toml in place
tspec ts show -t t2.ts.toml                    # shows strip = "symbols"
tspec ts backup -t t2.ts.toml                  # creates t2-001-<hash>.ts.toml
tspec ts set panic=abort -t t2.ts.toml         # modifies t2.ts.toml again
tspec ts show -t t2.ts.toml                    # shows panic + strip
tspec ts restore -t t2-001-<hash>.ts.toml      # restores t2.ts.toml to pre-panic state
tspec build -t t2-001-<hash>.ts.toml           # backup usable directly
```

**Status:** Done
