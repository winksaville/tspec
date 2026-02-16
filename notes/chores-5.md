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

### Status

Noting for future reference. No immediate action needed.
