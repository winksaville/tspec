# Chores-1

General maintenance tasks and considerations for the project.

## 20260202 - Initial Setup After Migration

Migrated xt from rlibc-x to standalone tspec repo using `git subtree split`.

### POP Support

Make tspec work with Plain Old Packages (single Cargo.toml without workspace). Previously errored with "could not find workspace root".

**Status:** Done - added `find_project_root()` and `is_pop()` helpers.

See [tspec-design.md](tspec-design.md#20260202---augment-vs-replace-cargo) for design decision.

### Rename Package and Binary

Renamed from `xt` to `tspec`:
- Cargo.toml `name = "tspec"`
- Binary is now `tspec` instead of `xt`
- CLI command name updated

**Status:** Done

### Update README

Remove rlibc-x specific references, update for standalone usage.

**Status:** Done

### Test in Isolation

Verify all tests pass without rlibc-x workspace context.

**Status:** Done - 79 tests pass

### Add tspec install --path

Add `tspec install --path <path>` command where path can be relative or absolute.

Wraps `cargo install --path` with path resolution:
- Add `Install` variant to `Commands` enum in cli.rs with `PathBuf` path arg
- Add handler in main.rs using `canonicalize()` for path resolution
- Optional `--force` flag for reinstall

**Status:** Done

### Rename tspec Subcommand to ts

The `tspec` subcommand creates awkward `tspec tspec list` invocations. Should rename to just `ts` so it becomes `tspec ts list`.

Currently in cli.rs:
```rust
#[command(alias = "ts")]
Tspec { ... }
```

Should become:
```rust
Ts { ... }
```

**Status:** Done

### Improve classify_crate

The `classify_crate()` function in `workspace.rs` uses name-matching which is brittle:

```rust
if name == "tspec" || name == "xt" || name == "xtask" {
    return CrateKind::BuildTool;
}
```

This could misclassify user crates named "tspec" or fail to recognize build tools with different names. Consider:
- Using Cargo.toml metadata (categories, keywords)
- Looking for `[[bin]]` with specific characteristics
- Making it configurable via workspace Cargo.toml

**Status:** Todo

### Remaining xt References

Some `xt` references intentionally remain for backwards compatibility:
- `workspace.rs:105` - recognizes "xt" as BuildTool (legacy workspaces)
- `workspace.rs` tests - verify "xt" classification still works

These ensure tspec works correctly when used in workspaces that still have an `xt` crate.

### CICD

Set up GitHub Actions for automated testing.

Created `.github/workflows/ci.yml` with three jobs:
- `test` - builds and runs tests
- `clippy` - runs clippy with `-D warnings`
- `fmt` - checks formatting

**Status:** Done
