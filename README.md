# tspec - Translation Spec Build System

A spec-driven build system for Rust that wraps cargo with configurable target triples, compiler flags, and linker options. Each tspec applies to a Cargo **package** — the unit with a `Cargo.toml`. If different crates need different compilation settings, they should be in separate packages.

**Status:** Active development. Works with both workspaces and single-package projects (i.e. Plain Old Packages, POPs).

## Installation

```bash
cargo install --path .
```

Or run directly:
```bash
cargo run -- build
```

## Contributing

**All non-trivial changes must be on a feature branch** — never commit directly to `main`. This applies even for solo development.

```bash
git checkout -b feat-my-feature    # Create a branch
# ... make changes, commit ...
git checkout main                  # Switch back
git merge --no-ff feat-my-feature  # Merge with merge commit (or squash-merge)
git push
```

Branch naming: `<type>-<description>` where type is `feat`, `fix`, `refactor`, `docs`, `chore`.

**Protect main on GitHub:** Enable branch protection (Settings > Branches) to block force pushes and optionally require PRs.

For the full feature workflow (version bumps, notes, merge steps, Claude Code conventions), see [CLAUDE.md](CLAUDE.md#feature-workflow).

## Usage

```bash
tspec build                                # Build current package (or all in workspace) with tspec.ts.toml if present
tspec build -p myapp                       # Build specific package with tspec.ts.toml if present
tspec build myapp                          # Same, using positional argument
tspec build .                              # Build the package in current directory
tspec build -p myapp -t tspec-opt          # Build with alternative spec tspec-opt.ts.toml
tspec build -p myapp -t tspec-opt.ts.toml  # Build with alternative spec tspec-opt.ts.toml
tspec build -p myapp -r -s                 # Build release, strip symbols with tspec.ts.toml if present
tspec run -p myapp                         # Build and run with tspec.ts.toml if present
tspec test -p myapp                        # Build and test with tspec.ts.toml if present
tspec build -w                             # Build all packages (even from inside a package dir)
tspec compare -p myapp                     # Compare binary sizes for a package
tspec compare -p myapp -t *.ts.toml        # Compare using shell-expanded glob
tspec compare                              # Compare all packages (workspace mode)
tspec compare -w                           # Force workspace mode from inside a package
tspec compare -w -f                        # Workspace mode, stop on first failure
```

The `-p` flag or positional argument specifies a package by name or path (defaults to current directory if in a package, otherwise all packages). Paths like `.` are resolved to the actual cargo package name. At a pure workspace root (no `[package]`), `.` means "all packages."
Use `-w, --workspace` to force all-packages mode even when inside a package directory.
The `-t` flag selects a tspec file; if omitted and `tspec.ts.toml` exists, it's used automatically.

**Note on `cmd` vs `cmd .`:** For passthrough commands (clean, clippy, fmt), `tspec cmd` passes no `-p` to cargo (operates on everything), while `tspec cmd .` resolves to `cargo cmd -p <name>` (package-specific). The difference is most visible with `clean`: `cargo clean` removes all of target/ while `cargo clean -p <name>` leaves shared metadata files. This matches cargo's own behavior. Use `tspec clean` or `tspec clean -w` for a full clean.

## tspec Files

Translation specs are TOML files (conventionally `*.ts.toml`) that configure builds:

```toml
# tspec.ts.toml - all supported fields shown

# Top-level high-level options (expand to lower-level cargo/rustc flags)
panic = "abort"                            # "unwind" (default), "abort", "immediate-abort" (nightly)
strip = "symbols"                          # "none" (default), "debuginfo", "symbols"
rustflags = ["-C", "relocation-model=static"]  # raw flags passed through to RUSTFLAGS

[cargo]
profile = "release"                        # "debug" (default), "release"
target_triple = "x86_64-unknown-linux-musl"
target_json = "path/to/custom-target.json" # custom target spec (auto-adds -Z json-target-spec)
unstable = ["panic-immediate-abort"]       # -Z flags (nightly only)
target_dir = "{name}"                      # per-spec target dir; supports {name} and {hash} placeholders
build_std = ["core", "alloc"]              # crates to rebuild with -Z build-std (nightly)

[cargo.config.profile.release]             # nested tables under [cargo.config] become individual
opt-level = "z"                            # --config KEY=VALUE args to cargo; any valid cargo
lto = true                                 # config key works (profile, build, target, etc.)
codegen-units = 1

[linker]
args = ["-static", "-nostartfiles"]

[linker.version_script]                    # symbol visibility control (enables --gc-sections)
global = ["_start"]                        # symbols to keep
local = "*"                                # default: "*" (hide everything else)
```

The top-level `panic` sets both cargo `-Z` and rustc `-C` flags automatically.

### Key Concepts

- **One tspec per package** - A tspec applies to all crates (targets) within a Cargo package. For per-crate control, use separate packages in a workspace.
- **tspec.ts.toml is optional** - Packages without one get plain `cargo build/test`
- **Generated build.rs** - tspec generates temporary build.rs for scoped linker flags
- **Spec comparison** - Compare binary sizes across different specs; always shows both `cargo --release` and `cargo --release-strip` baselines. Supports `-w` for all-packages mode — per-package tables are collected and printed together at the end. With a single package, only its comparison table is shown

### How Linker Flags Work

tspec generates a temporary `build.rs` to apply linker flags with binary-scoped directives:

```rust
// Generated by tspec - deleted after build
fn main() {
    println!("cargo:rustc-link-arg-bin=myapp=-static");
    println!("cargo:rustc-link-arg-bin=myapp=-nostdlib");
}
```

The only other alternative is to add these linker flags to RUSTFLAGS, but that
affects ALL compilations (including dependency build scripts), causing crashes
with flags like `-nostartfiles`. The generated build.rs scopes flags to just
the binary.

### Cargo Config Passthrough (`cargo.config`)

The `[cargo.config]` section is a general-purpose passthrough to cargo's `--config KEY=VALUE` mechanism. Any nested tables under `[cargo.config]` are flattened into dotted keys and passed as `--config` args. This lets you override any [cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html) value at the command line.

**Profile settings** are the most common use case:

```toml
[cargo.config.profile.release]
opt-level = "z"
codegen-units = 1
lto = true
```

This produces: `cargo --config profile.release.opt-level="z" --config profile.release.codegen-units=1 --config profile.release.lto=true`

**Other cargo config keys** work too:

```toml
[cargo.config.target.x86_64-unknown-linux-gnu]
linker = "clang"
runner = "qemu-x86_64"
```

**Note:** Do not use `build.rustflags` in `[cargo.config]`. tspec sets the `RUSTFLAGS` env var (from top-level `rustflags`, `panic`, and `strip`), which takes precedence over `--config build.rustflags` per cargo's precedence rules. Use the top-level `rustflags` array instead.

**Profile restrictions:** Only `profile.debug` and `profile.release` are supported. tspec will error if other profile names (e.g. `profile.custom`) are used in `[cargo.config]`.

**Flat dotted keys** also work — TOML treats them equivalently:

```toml
[cargo.config]
"profile.release.opt-level" = "z"
```

## tspec Management (ts subcommand)

Manage spec files without manual TOML editing. All commands that modify files preserve comments and formatting.

| Command | Description |
|---------|-------------|
| `ts list` | List tspec files found in the workspace or a specific package |
| `ts show` | Display the TOML contents of a tspec file |
| `ts hash` | Print the content hash (used in backup filenames) of a tspec |
| `ts new` | Create a new tspec with defaults, or copy an existing one with `-f` (byte-for-byte) |
| `ts set` | Set a scalar value or replace an entire array |
| `ts unset` | Remove a field (scalar or array) from a tspec entirely |
| `ts add` | Add items to an array field (append or insert at position) |
| `ts remove` | Remove items from an array field (by value or by index) |
| `ts backup` | Create a versioned backup copy (`name-NNN-hash.ts.toml`) |
| `ts restore` | Restore a tspec by copying a versioned backup back to its original name |

```bash
# List / show / hash
tspec ts list                            # List all tspec files
tspec ts list -p myapp                   # List tspec files for a package
tspec ts show -p myapp                   # Show all tspec contents
tspec ts show -p myapp -t tspec-opt      # Show specific tspec-opt.ts.toml
tspec ts hash -p myapp                   # Show content hash of tspec.ts.toml

# Create new tspec files
tspec ts new -p myapp                    # Create tspec.ts.toml with defaults
tspec ts new experiment -p myapp         # Create experiment.ts.toml
tspec ts new opt2 -p myapp -f tspec-opt  # Copy from existing spec (byte-for-byte)

# Set scalar values
tspec ts set strip symbols -p myapp              # Set a scalar field
tspec ts set cargo.profile release               # No quoting needed

# Set (replace) entire arrays — each element is a separate arg
tspec ts set linker.args -static -nostdlib        # Replace array
tspec ts set cargo.build_std core alloc           # Replace array
tspec ts set rustflags -C relocation-model=static # Replace array

# Add items to arrays (append by default)
tspec ts add linker.args -Wl,--gc-sections        # Append one item
tspec ts add linker.args -nostdlib -pie            # Append multiple items
tspec ts add -i 0 linker.args -nostartfiles        # Insert at position 0

# Remove items from arrays
tspec ts remove linker.args -nostdlib              # Remove by value
tspec ts remove linker.args -static -pie           # Remove multiple by value
tspec ts remove -i 2 linker.args                   # Remove by index

# Remove a field entirely
tspec ts unset strip -p myapp                      # Remove scalar field
tspec ts unset linker.args -t tspec-opt            # Remove array field

# Backup and restore (byte-for-byte copies)
tspec ts backup -p myapp                 # Backup tspec.ts.toml → tspec-001-abcd1234.ts.toml
tspec ts backup -p myapp -t tspec-opt    # Backup tspec-opt.ts.toml
tspec ts restore -t t1-001-abcd1234      # Restore t1.ts.toml from backup
```

Backups are valid spec files and can be used directly with `-t`.

`ts set`, `ts unset`, `ts add`, and `ts remove` use `toml_edit` for surgical editing — comments and formatting are preserved.
`ts backup`, `ts restore`, and `ts new -f` use byte-for-byte file copy — comments are preserved exactly.

### Supported keys

Keys use two forms: `FIELD` for top-level fields (`panic`, `strip`, `rustflags`) or `TABLE.FIELD` for nested fields (`cargo.profile`, `linker.args`).

| Key | Type | Values |
|-----|------|--------|
| `panic` | scalar | `unwind`, `abort`, `immediate-abort` |
| `strip` | scalar | `none`, `debuginfo`, `symbols` |
| `rustflags` | array | e.g. `-C relocation-model=static` |
| `cargo.profile` | scalar | `debug`, `release` |
| `cargo.target_triple` | scalar | any string |
| `cargo.target_json` | scalar | any path |
| `cargo.target_dir` | scalar | any string (supports `{name}`, `{hash}`) |
| `cargo.unstable` | array | e.g. `panic-immediate-abort` |
| `cargo.config` | table | arbitrary `--config KEY=VALUE` passthrough; supports nested tables (e.g. `[cargo.config.profile.release]`) |
| `cargo.build_std` | array | e.g. `core alloc` |
| `linker.args` | array | e.g. `-static -nostdlib` |

### Array operations

Four orthogonal commands operate on arrays at two levels:

| Level | Command | Purpose |
|-------|---------|---------|
| Field-level | `ts set` | Replace entire array with new values |
| Field-level | `ts unset` | Remove the array field entirely |
| Item-level | `ts add` | Append items (or insert at position with `-i`) |
| Item-level | `ts remove` | Remove items by value (or by index with `-i`) |

```bash
tspec ts set linker.args -static -nostdlib   # replace all with these
tspec ts unset linker.args                   # clear the field
tspec ts add linker.args -Wl,--gc-sections   # append (skips duplicates)
tspec ts add -i 0 linker.args -nostartfiles  # insert at position
tspec ts remove linker.args -static          # remove by value
tspec ts remove -i 2 linker.args             # remove by index
```

Values starting with `-` work without quoting thanks to `allow_hyphen_values`.
Use `--` as an escape hatch if a value collides with a flag like `-p`, `-t`, or `-i`.

Note: Use `ts` as the subcommand (short for "tspec management").

## Testing

```bash
tspec test -p tspec           # Run tspec tests
tspec test -p tspec-build     # Run tspec-build tests
```

## Project Structure

```
tspec/                  # Workspace root (also the main tspec package)
  Cargo.toml            # [workspace] + [package] — workspace with root package
  src/
    lib.rs              # Library root, exposes modules
    main.rs             # Entry point, dispatch
    cli.rs              # Clap CLI definitions
    types.rs            # Spec parameter types
    tspec.rs            # Spec loading/saving/hashing
    find_paths.rs       # Project/package/tspec/binary path discovery
    workspace.rs        # Workspace package discovery
    cargo_build.rs      # Build command + generated build.rs
    run.rs              # Run command implementation
    testing.rs          # Test command implementation
    compare.rs          # Compare command (size comparison)
    all.rs              # Batch operations (build_all, run_all, test_all, compare_all)
    cmd/                # Command implementations (one file per command)
    ts_cmd/             # Tspec management subcommands
    binary.rs           # Binary operations (strip, size)
  tests/
    data/               # Test fixtures (TOML specs)
    tspec_test.rs       # Integration tests
  tspec-build/          # Library crate for build.rs integration
    src/lib.rs          # Reads TSPEC_SPEC_FILE env var at build time
  notes/                # Design docs and todo tracking
```

## Development

See [notes/todo.md](notes/todo.md) for current tasks.

### Verification

```bash
tspec test -p tspec
tspec test -p tspec-build
tspec clippy
tspec fmt --check
```

## Origin

Originally developed as `xt` within the [rlibc-x](https://github.com/user/rlibc-x) project, then extracted to a standalone tool. The git history was preserved using `git subtree split`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
