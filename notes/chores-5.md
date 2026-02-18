# Chores-5

## 20260216 - Future: config_key_value and rustc field migration

### Context

`[cargo.config_key_value]` is now implemented, providing a general-purpose mechanism for
passing `--config KEY=VALUE` args to cargo. This enables per-package profile overrides
without bleeding settings through RUSTFLAGS.

### Open questions

- **Auto-scoping:** Should tspec automatically scope config keys with
  `profile.*.package.<name>` so users don't have to write the full path? Currently users
  write exact cargo config keys — no magic.
- **`[rustc]` field migration:** Several `[rustc]` fields (`opt_level`, `lto`,
  `codegen_units`) duplicate what cargo profiles express. These could migrate to
  `config_key_value` entries, but that's a separate follow-up. The existing fields work
  fine via RUSTFLAGS for now.
- **High-level fields (`panic`, `strip`):** These expand to both cargo and rustc flags.
  Moving them to `config_key_value` would lose the multi-flag expansion. They likely stay
  as top-level fields.

### Finding: RUSTFLAGS vs --config produce same-size but non-identical binaries

Tested with `tspec-build` library crate using two equivalent specs:
- `tspec.opt.ts.toml` — uses `rustc.opt_level = "z"` and `rustc.codegen_units = 1` (RUSTFLAGS)
- `tspec.opt-kv.ts.toml` — uses `[cargo.config_key_value]` with equivalent `--config` args

Results:
- Binary sizes: **identical** (1,117,880 bytes both)
- Binary content: **differs** (~72K of ~1.1M bytes, ~6.5%)
- Changing `target_dir` alone does NOT cause differences (confirmed byte-identical)
- The difference is solely from RUSTFLAGS `-C opt-level=z` vs `--config 'profile.release.opt-level="z"'`

The two mechanisms produce equivalently optimized but not bit-identical binaries.
Cause is unclear — possibly different flag ordering, different cargo-internal rustc
invocation paths, or non-deterministic codegen. Not a practical concern since the
optimization effect is the same.

### Status

Done — see next section.

## 20260216 - Add glob support for `-t` flag on build, run, test

### Context

The compare command already supported glob patterns for `-t` and `-w`/`--workspace` for
all-packages mode. Build, run, and test commands only accepted a single `-t` value and
had no `-w` flag.

### Changes

- Changed `-t`/`--tspec` from `Option<String>` to `Vec<String>` with `num_args=1..`
  on build, run, and test commands, matching compare's pattern
- When multiple specs match a glob, the operation executes once per spec in sequence
- Block `-t` in all-packages mode (`-w`) with a clear error
- Added clap parsing tests to all three commands

### Status

Done — released in v0.11.3.

## 20260216 - Remove `rustc.opt_level`, `rustc.lto`, `rustc.codegen_units` — migrate to `config_key_value`

### Context

These three `[rustc]` fields duplicate what `[cargo.config_key_value]` can express via
`cargo --config`. We confirmed they produce identical binary sizes. Since we're the only
users, we can remove them without deprecation. `[rustc]` section stays with `build_std`
and `flags`.

### Changes

1. **types.rs:** Deleted `OptLevel` enum; removed `opt_level`, `lto`, `codegen_units`
   from `RustcConfig` (kept `build_std`, `flags`)
2. **cargo_build.rs:** Removed RUSTFLAGS generation for the three fields; removed
   `OptLevel` import
3. **ts_cmd/edit.rs:** Removed 3 `FIELD_REGISTRY` entries, 3 `validate_value` arms,
   `parse_scalar_value` special cases; updated/deleted affected unit tests
4. **ts_cmd/set.rs:** Deleted `set_rustc_lto`, `set_rustc_opt_level`, `set_codegen_units` tests
5. **ts_cmd/unset.rs:** Rewrote `unset_nested_field` test to use `rustc.build_std`/`flags`
6. **cmd/ts.rs:** Changed `"rustc.lto"` doc examples to `"cargo.profile"`
7. **Spec files:** Migrated `opt_level`/`codegen_units` to `[cargo.config_key_value]`
   in `tspec.release.ts.toml`, `tspec.musl.ts.toml`, `tspec-build/tspec.opt.ts.toml`
8. **Test fixtures:** Migrated `tests/data/minimal.toml`; updated `tests/tspec_test.rs`
   `load_minimal_spec` to check `config_key_value` instead of `rustc.opt_level`

### Status

Done.

## 20260217 - Fix: test race condition with `std::env::set_var` / `remove_var`

### Problem

`tspec test` intermittently failed on the `resolve_spec_path_none_with_env` test in
`tspec-build`. The failure was not reproducible when running `tspec test -p tspec-build`
alone — only when both packages ran together via `tspec test`.

### Root cause

Rust runs tests in parallel on multiple threads within the same process.
`std::env::set_var()` and `std::env::remove_var()` are process-wide — they mutate
shared global state. Two tests that manipulate the same env var (`TSPEC_SPEC_FILE`)
race each other:

- `resolve_spec_path_none_with_env` — sets `TSPEC_SPEC_FILE`, calls function, removes it
- `resolve_spec_path_none_without_env` — removes `TSPEC_SPEC_FILE`, asserts `None`

If the "without" test runs between the set and remove of the "with" test, it sees the
var and fails. Or vice versa. This is a well-known Rust testing pitfall — `set_var` is
marked `unsafe` in Rust 2024 edition precisely because of this.

### Solutions considered

1. **Static Mutex** — wrap env-var tests so they run one at a time. Works but easy to
   forget for new tests, and doesn't prevent the class of bug.
2. **`serial_test` crate** — adds `#[serial]` attribute. Clean syntax but adds a
   dependency and still relies on developers remembering to annotate.
3. **`--test-threads=1`** — heavy-handed, slows down all tests.
4. **`_inner` pattern (chosen)** — extract a pure function that takes env values as
   parameters. The public function reads env vars and delegates. Tests call `_inner`
   directly with their own values — no shared state, no race, impossible to flake.
5. **Temp directory copies (for future integration tests)** — when tests need to
   modify real package layouts (Cargo.toml, spec files, source), each test copies
   the fixture into its own `tempdir()`. No shared filesystem state, parallel-safe,
   cleanup is automatic via `Drop`.

### Principle

Push side effects (env var reads, file I/O) to the edges. Keep core logic pure and
parameterized. Tests exercise the logic without touching the real world. For filesystem
mutations, each test owns a private copy.

### Changes

- **tspec-build/src/lib.rs:** Split `resolve_spec_path` into a thin env-reading wrapper
  and `resolve_spec_path_inner` that takes `manifest_dir` and `env_spec_file` as
  `Option<&str>` parameters. Rewrote all three `resolve_spec_path_*` tests and
  `emit_from_reads_spec_file` to call `_inner` directly — no `set_var`/`remove_var`.

### Status

Done.

## 20260217 - Refactor cargo runner functions into unified fn

### Context

Three functions share nearly identical structure:
- `cargo_build::build_package` — builds with optional spec, returns `BuildResult`
- `testing::test_package` — tests with optional spec, returns `()`
- `cargo_build::plain_cargo_build_release` — builds without spec, returns `BuildResult`

They all follow the same skeleton: find workspace/package/tspec, clean stale build.rs,
optionally load spec, generate build.rs if needed, construct `Command` (with optional
`+nightly`), run, clean up, warn about stale files.

`plain_cargo_build_release` is just `build_package` with `tspec=None` and `release=true`.
`test_package` differs only in the subcommand (`test` vs `build`) and has its own
duplicate `requires_nightly()` function in `testing.rs`.

### Plan

Unify into a single `run_cargo` function that takes the subcommand as a parameter.
`plain_cargo_build_release` becomes a thin call. `test_package` becomes a wrapper that
calls the unified function and discards `BuildResult`. The duplicate `requires_nightly()`
in `testing.rs` is eliminated.

### Differences to reconcile

| Aspect | `build_package` | `test_package` | `plain_cargo_build_release` |
|---|---|---|---|
| Subcommand | `build` | `test` | `build` |
| Spec | optional | optional | never |
| Returns | `BuildResult` | `()` | `BuildResult` |
| `+nightly` | via `build_cargo_command()` | inlined `requires_nightly()` | no (no spec) |
| build.rs gen | yes (checks bin target) | yes (no bin check) | no |

### Status

Not started.

## 20260217 - Remove `[rustc]` section, promote `build_std`, add `rustflags`

### Context

The `[rustc]` section has two remaining fields: `build_std` (actually a cargo `-Z` flag,
not a rustc flag) and `flags` (raw RUSTFLAGS passthrough). `build_std` belongs under
`[cargo]` since it triggers `+nightly` and `-Z build-std=...`. `flags` becomes a
top-level `rustflags` array — a simple escape hatch mirroring `RUSTFLAGS` semantics.

### Before → After

```toml
# Before
[rustc]
build_std = ["core", "alloc"]
flags = ["-C", "some-thing"]

# After
[cargo]
build_std = ["core", "alloc"]

rustflags = ["-C", "some-thing"]
```

### Changes

1. **types.rs:** Move `build_std` to `CargoConfig`, add `rustflags` to `Spec`, delete `RustcConfig`
2. **cargo_build.rs:** Update all `spec.rustc` references to new locations
3. **testing.rs:** Update duplicate `requires_nightly()` to use `spec.cargo.build_std`
4. **ts_cmd/edit.rs:** Update field registry and tests
5. **ts_cmd/set.rs, unset.rs:** Update tests
6. **tspec.rs:** Update tests (remove `RustcConfig` references)
7. **tests/tspec_test.rs:** Update integration tests
8. **README.md, CLAUDE.md:** Update documentation

### Status

Done — released in v0.11.6. Also renamed `config_key_value` to `config` with nested table support (dev2).

## 20260218 - Add custom profile support

### Context

Currently `cargo.profile` only accepts `"debug"` or `"release"` (a Rust enum). Cargo supports
custom profiles defined in `Cargo.toml` (e.g., `[profile.release-small]` with `inherits = "release"`),
selected via `cargo build --profile <name>`. tspec should support selecting and overriding these
custom profiles.

Key insight: Custom profile **definitions** must live in `Cargo.toml` (cargo requirement — they
need `inherits`). tspec's role is **selecting** a profile and **overriding** its settings via
`cargo.config`.

Cargo's profile-to-directory mapping: `dev` → `target/debug/`, `release` → `target/release/`,
custom → `target/<name>/`.

### Plan

Multi-step (`-devN` series), version `0.12.0`.

**Step 1 (dev1):** Core type change — replace `Profile` enum with `String`, update command
generation (`"debug"`/`"dev"` → no flag, anything else → `--profile <name>`), update path
resolution (`profile_dir_name()` helper), relax validation.

**Step 2 (dev2):** CLI `--profile <name>` flag on build/run/test commands, thread through
call chain, replace `release: bool` with `Option<String>`.

**Step 3:** Final release — remove `-dev` suffix.

### Changes

**Step 1 (dev1):** Replaced `enum Profile { Debug, Release }` with `Option<String>`.
Updated `apply_spec_to_command()` to emit `--profile <name>` for any non-debug/dev profile.
Added `profile_dir_name()` helper for `dev` → `debug` directory mapping.
Removed `validate_config_profiles()` — custom profiles in `[cargo.config]` are now accepted.
Relaxed `validate_value("cargo.profile", ...)` to accept any string.

**Step 2 (dev2):** Added `--profile <name>` flag to build/run/test commands (`conflicts_with = "release"`).
Replaced `release: bool` with `cli_profile: Option<&str>` throughout the call chain:
`build_package()`, `test_package()`, `apply_spec_to_command()`, `get_binary_path()`,
`get_binary_path_simple()`, `build_all()`, `test_all()`, `run_all()`.
Simplified `plain_cargo_build_release()` to a thin wrapper: `build_package(name, None, Some("release"))`.

### Status

Done — released in v0.12.0.
