# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**tspec** is a spec-driven build system wrapper for Rust that sits on top of cargo. It allows configuring builds via translation spec files (TOML-based) with support for target triples, compiler flags, linker options, and high-level build options like panic strategies and symbol stripping.

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

### CargoPassthrough Pattern

Commands that wrap cargo subcommands implement the `CargoPassthrough` trait:

```rust
pub trait CargoPassthrough {
    fn subcommand(&self) -> &str;           // e.g., "clean", "test"
    fn args(&self) -> Vec<OsString>;        // Arguments for cargo
    fn execute(&self, project_root: &Path) -> Result<ExitCode>;  // Default or custom impl
}
```

- **Simple passthroughs** (Clean): Use default `execute()` implementation
- **Complex commands** (Test, Build, Run): Override `execute()` with custom logic

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
  - `mod.rs` - CargoPassthrough trait and re-exports
  - `build.rs`, `clean.rs`, `clippy.rs`, `compare.rs`, `fmt.rs`, `install.rs`, `run.rs`, `test.rs`, `version.rs` - Individual commands
- `cli.rs` - Clap CLI definitions
- `types.rs` - Spec types (CargoConfig, RustcConfig, LinkerConfig)
- `tspec.rs` - Spec loading/saving/hashing
- `cargo_build.rs` - Build orchestration with spec application
- `workspace.rs` - Workspace member discovery
- `all.rs` - Batch operations (build_all, test_all, run_all)

### Translation Spec Structure

Specs are TOML files (`*.ts.toml`) with three sections:
- `[cargo]` - profile, target_triple, target_json, unstable flags
- `[rustc]` - opt_level, panic, lto, codegen_units, build_std, flags
- `[linker]` - args, version_script

## Conventions

- **Rust Edition:** 2024
- **Commit style:** Conventional commits (feat:, docs:, refactor:)
- **Naming:** POP (Plain Old Package) refers to single-crate projects; tspec treats them as trivial workspaces

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
