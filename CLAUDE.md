# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**tspec** is a spec-driven build system wrapper for Rust that sits on top of cargo. It configures builds at the **package** level (one tspec per Cargo package) via translation spec files (TOML-based) with support for target triples, compiler flags, linker options, and high-level build options like panic strategies and symbol stripping. For per-crate control, use separate packages in a workspace.

## Build Commands — Dogfood tspec

**Always use `tspec` itself for development.** Never use bare `cargo test`, `cargo clippy`, `cargo fmt`, or `cargo install` — use the tspec equivalents. This is how we catch real-world issues.

```bash
tspec test -p tspec            # Run tests (NEVER cargo test)
tspec clippy                   # Run lints (NEVER cargo clippy)
tspec fmt --check              # Check formatting (NEVER cargo fmt --check)
tspec install --path .         # Install from local source (NEVER cargo install)
```

**After code changes pass tests, immediately install the new version** with `tspec install --path .` so subsequent commands use the latest binary. Do not defer installation — the sooner we run the real binary, the sooner we catch issues.

## Architecture

### Execute Trait Pattern

Commands implement the `Execute` trait:

```rust
pub trait Execute {
    fn execute(&self, project_root: &Path) -> Result<ExitCode>;
}

// Helper for simple cargo passthroughs
pub fn execute_cargo_subcommand(subcommand: &str, args: &[OsString], project_root: &Path) -> Result<ExitCode>
```

- **Simple passthroughs** (Clean, Clippy, Fmt): Use `execute_cargo_subcommand()` helper
- **Custom commands** (Build, Run, Test, Compare, Install, Version): Implement own logic

Command structs derive `clap::Args` and are used directly in the `Commands` enum:
```rust
enum Commands {
    Clean(CleanCmd),
    Test(TestCmd),
    // ...
}
```

### Key Modules

- `cmd/` - Command implementations (one file per command)
  - `mod.rs` - Execute trait, execute_cargo_subcommand helper, re-exports
  - `build.rs`, `clean.rs`, `clippy.rs`, `compare.rs`, `fmt.rs`, `install.rs`, `run.rs`, `test.rs`, `version.rs` - Individual commands
- `cli.rs` - Clap CLI definitions
- `types.rs` - Spec types (CargoConfig, RustcConfig, LinkerConfig)
- `tspec.rs` - Spec loading/saving/hashing, `copy_spec_snapshot()` for byte-for-byte backups
- `cargo_build.rs` - Package build orchestration with spec application
- `workspace.rs` - Workspace package discovery
- `all.rs` - Batch operations (build_all, test_all, run_all)
- `ts_cmd/edit.rs` - `toml_edit` helpers: field registry, `set_field()`, `unset_field()`, `add_items()`, `remove_items_by_value()`, `remove_item_by_index()`, validation
- `ts_cmd/set.rs` - `ts set` command (scalar or replace entire array)
- `ts_cmd/unset.rs` - `ts unset` command (remove field entirely)
- `ts_cmd/add.rs` - `ts add` command (append or insert at position)
- `ts_cmd/remove.rs` - `ts remove` command (by value or by index)

### Three Write Strategies

| Operation | Strategy | Why |
|---|---|---|
| load, hash, build | serde (`toml`) | Need typed `Spec` struct for build logic |
| set, unset, add, remove | `toml_edit` (`DocumentMut`) | Surgical edit preserving comments/formatting |
| backup, restore, new -f | raw `fs::copy` | Exact byte-for-byte preservation |

### Translation Spec Structure

Specs are TOML files (`*.ts.toml`) with three sections:
- `[cargo]` - profile, target_triple, target_json, unstable flags
- `[rustc]` - opt_level, panic, lto, codegen_units, build_std, flags
- `[linker]` - args, version_script

## Conventions

- **Rust Edition:** 2024
- **Commit style:** Conventional commits (feat:, docs:, refactor:)
- **Naming:** POP (Plain Old Package) refers to single-package projects (no workspace); tspec treats them as trivial workspaces
- **Granularity:** tspec operates at the Cargo package level, not the crate level. "Package" = directory with Cargo.toml. A package may contain multiple crates (targets), but they all share one tspec.
- **Markdown refs:** Multiple references use `[1],[2]` not `[1,2]` or `[1][2]` (both break in markdown)

## Feature Workflow

**Before starting feature or fix work:**
1. Create a branch: `git checkout -b <type>-<short-description>` (e.g., `fix-compare-optional-p`)
2. Create a dated entry in `notes/chores-N.md` with context and plan
3. Update `notes/todo.md` to move items to In Progress
4. Bump version to `X.Y.Z-dev` in `Cargo.toml`
5. Commit the above as a chore marker commit before starting code changes
6. Use todo list to track progress during implementation

**Branch naming:** `<type>-<description>` where type is `feat`, `fix`, `refactor`, `docs`, `chore`

## Verification Workflow

**After code changes, run verification and install immediately:**
```bash
tspec test -p tspec            # Run tests to verify changes with "old" binary if all pass:
tspec install --path .         # Install ASAP so we dogfood the new binary
tspec test -p tspec            # Run tests again with the new binary to potentially catch issues early
tspec clippy
tspec fmt --check
```

**After committing code, remind about .claude/ files:**
```
Committed abc123.

Remember to commit .claude/ session files.
```

**On next prompt after a commit+reminder:** Check `git log -1 --name-only` to see if `.claude/` was included in a commit after the code commit. If not, ask: "Did you forget to commit .claude sessions?"

## Git Operations Claude Cannot Perform

Due to circular references with `.claude/` session files (which record Claude's behavior), Claude cannot perform these git operations:

- `git checkout` - switching branches affects session files
- `git merge` - merging affects session files
- `git rebase` - rebasing affects session files
- Amending commits with `.claude/` files

The user must perform these operations manually. Claude can only make commits (excluding `.claude/` files) and remind the user to handle the rest.
