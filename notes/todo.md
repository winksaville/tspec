# Todo

## In Progress

## Todo
- Add benchmark support, especially cold-start vs hot-start build timing
- Add database for collecting build data over time (could store tspecs, possibly replace backup/restore)
- Investigate `-static` vs `dynamic-linking=false` size difference (partially explained); note: glibc + `-static` segfaults (glibc not designed for static linking), consider musl for static builds [11]
- Warn on obvious spec misconfigurations: `linker.args` on lib-only packages (no bin target), `-static` on glibc systems (segfaults), panic=abort specs used with test, etc. Specs are copy/pastable so users will apply bin-oriented specs to lib crates

## Done

See older [done.md](done.md)

- Add `--manifest-path` / `--mp` flag; popws-3p fixture with tests [42]
- Parse cargo test output: per-package counts in summary, 0-test guard, noise filtering, fail fixtures [41]
- Remove `classify_package`/`PackageKind` and `is_build_tool`; simplify workspace to just members [40]
- Add test fixtures (POP, POP+WS, POWS) and integration tests; add --test/trailing args to tspec test [38]
- Pass `-v`/`-vv`/`-j N` through to cargo via `CargoFlags` struct [37]
- Unified cargo runner + `--verbose`/`-v`/`-vv` support [36]
- Add `toolchain` field to translation specs [35]
- Refactor summary printers into shared `print_summary_table()` [34]
- Allow `-t` glob patterns in all-packages mode for workspaces [33]
- Add custom profile support and CLI `--profile` flag [32]
- Remove `[rustc]` section, promote `build_std` to `[cargo]`, add top-level `rustflags`; rename `config_key_value` to `config` with nested table support [31]
- Add glob support for `-t` flag on build, run, test commands [29]
- Remove `rustc.opt_level`, `rustc.lto`, `rustc.codegen_units` — migrate to `config_key_value`; fix tspec-build test race condition [28]
- Add `[cargo.config_key_value]` for `--config KEY=VALUE` args [25],[26],[27]

[28]: chores-5.md#20260216---remove-rustcopt_level-rustclto-rustccodegen_units--migrate-to-config_key_value
[29]: chores-5.md#20260216---add-glob-support-for--t-flag-on-build-run-test
[11]: chores-1.md#investigate--static-vs-dynamic-linkingfalse-size-difference
[25]: chores-4.md#20260215---design-profile-support-and-tspec-section-scoping
[26]: chores-4.md#20260216---design-passing-tspec-fields-via-buildrs-vs-cargo---config
[27]: chores-4.md#20260216---implement-cargoconfig_key_value-support
[30]: chores-5.md#20260217---refactor-cargo-runner-functions-into-unified-fn
[31]: chores-5.md#20260217---remove-rustc-section-promote-build_std-add-rustflags
[32]: chores-5.md#20260218---add-custom-profile-support
[33]: chores-5.md#20260218---allow--t-glob-patterns-in-all-packages-mode
[34]: chores-5.md#20260218---refactor-summary-printers
[35]: chores-5.md#20260219---add-toolchain-field
[37]: chores-5.md#20260219---cargoflags-pass--v-vv-j-through-to-cargo
[36]: chores-5.md#20260219---unified-cargo-runner--verbose
[40]: chores-6.md#20260220---remove-classify_package-and-packagekind
[38]: chores-6.md#20260219---test-infrastructure-fixture-workspaces-and-test-args
[39]: chores-6.md#future-manifest-path--path-flag
[42]: chores-6.md#20260223---add---manifest-path-flag-with---mp-alias
[41]: chores-6.md#20260220---parse-cargo-test-output-for-counts-and-filtering
