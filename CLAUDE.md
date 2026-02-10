# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**tspec** is a spec-driven build system wrapper for Rust that sits on top of cargo. It configures builds at the **package** level (one tspec per Cargo package) via translation spec files (TOML-based) with support for target triples, compiler flags, linker options, and high-level build options like panic strategies and symbol stripping. For per-crate control, use separate packages in a workspace.

## Build Commands

Ensure a recent tspec is installed:
```bash
cargo install --path .         # Install from local source
```

Development workflow:
```bash
tspec test -p tspec            # Run tests
tspec clean                    # Clean build artifacts
tspec clippy                   # Run lints
tspec fmt --check              # Check formatting
```

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
- `tspec.rs` - Spec loading/saving/hashing
- `cargo_build.rs` - Package build orchestration with spec application
- `workspace.rs` - Workspace package discovery
- `all.rs` - Batch operations (build_all, test_all, run_all)

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

## Workflow

**Before committing, run verification:**
```bash
tspec test -p tspec
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
