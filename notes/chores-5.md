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
