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

Noting for future reference. No immediate action needed.
