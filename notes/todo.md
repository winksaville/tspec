# Todo

## In Progress

## Todo
- Commands like build, run ... should support glob and -w like in compare
- Add a permanent test workspace for integration testing (external repo or embedded?)
- Add benchmark support, especially cold-start vs hot-start build timing
- Add database for collecting build data over time (could store tspecs, possibly replace backup/restore)
- Investigate `-static` vs `dynamic-linking=false` size difference (partially explained); note: glibc + `-static` segfaults (glibc not designed for static linking), consider musl for static builds [11]
- Improve `classify_crate` - using name alone is brittle [4]

## Done

See older [done.md](done.md)

- Remove `rustc.opt_level`, `rustc.lto`, `rustc.codegen_units` — migrate to `config_key_value`; fix tspec-build test race condition [28]
- Add `[cargo.config_key_value]` for `--config KEY=VALUE` args [25],[26],[27]

[28]: chores-5.md#20260216---remove-rustcopt_level-rustclto-rustccodegen_units--migrate-to-config_key_value
[4]: chores-1.md#improve-classify_crate
[11]: chores-1.md#investigate--static-vs-dynamic-linkingfalse-size-difference
[25]: chores-4.md#20260215---design-profile-support-and-tspec-section-scoping
[26]: chores-4.md#20260216---design-passing-tspec-fields-via-buildrs-vs-cargo---config
[27]: chores-4.md#20260216---implement-cargoconfig_key_value-support
