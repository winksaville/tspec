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

### CargoPassthrough Trait for Wrapper Commands

After adding `install`, noticed pattern: many commands just wrap cargo subcommands (clean, install, clippy, fmt, check, doc). A trait with default implementation could reduce boilerplate.

**Approaches Considered:**

**1. Builder/Helper struct:**
```rust
struct CargoWrapper {
    command: &'static str,
    package: Option<String>,
    release: bool,
    apply_tspec: bool,
}

impl CargoWrapper {
    fn new(command: &'static str) -> Self { ... }
    fn package(mut self, p: Option<String>) -> Self { ... }
    fn with_tspec(mut self, tspec: Option<&str>) -> Self { ... }
    fn run(self) -> Result<ExitCode> { ... }
}

// Usage:
CargoWrapper::new("clippy")
    .package(package)
    .run()?;
```

**2. Trait with default implementation (preferred):**
```rust
trait CargoPassthrough {
    fn subcommand(&self) -> &str;
    fn args(&self) -> Vec<OsString>;
    fn execute(&self) -> Result<ExitCode> { /* default impl */ }
}
```

**3. Macro:**
```rust
cargo_passthrough!(Clippy, "clippy");
cargo_passthrough!(Fmt, "fmt");
```

**Selected Design (Trait):**

```rust
trait CargoPassthrough {
    fn subcommand(&self) -> &str;
    fn args(&self) -> Vec<OsString>;
    fn workdir(&self) -> Option<&Path> { None }

    // Default implementation - shared by all
    fn execute(&self) -> Result<ExitCode> {
        let mut cmd = Command::new("cargo");
        cmd.arg(self.subcommand());
        cmd.args(self.args());
        if let Some(dir) = self.workdir() {
            cmd.current_dir(dir);
        }
        let status = cmd.status()
            .with_context(|| format!("failed to run cargo {}", self.subcommand()))?;
        if status.success() {
            Ok(ExitCode::SUCCESS)
        } else {
            bail!("cargo {} failed", self.subcommand());
        }
    }
}
```

Each wrapper command implements the trait with minimal code:

```rust
struct ClippyCmd { package: Option<String> }

impl CargoPassthrough for ClippyCmd {
    fn subcommand(&self) -> &str { "clippy" }
    fn args(&self) -> Vec<OsString> {
        match &self.package {
            Some(p) => vec!["-p".into(), p.into()],
            None => vec![],
        }
    }
}
```

**Considerations:**
- Shared code is ~10 lines (execute), each wrapper ~10 lines
- Worth doing if adding several wrappers (clippy, fmt, check, doc)
- Two categories of commands: simple passthrough vs tspec-aware (build/run/test)
- Could also use builder pattern or macro, but trait is idiomatic Rust

**Status:** Todo

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
