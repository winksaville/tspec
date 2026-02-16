# Done

- Remove `rustc.panic` (duplicate of global panic) [24]
- Add `tspec compare -w/--workspace` for all-packages mode [22]
- Detect and remove stale tspec-generated build.rs [20]
- Always include `cargo --release` baseline in compare [19]
- Fix compare: optional `-p` and glob `-t` handling [18]
- Design: `tspec-build` library crate for linker.args when package has its own build.rs [21]
- Orthogonal `ts set`/`add`/`remove` with separate key and value args [17]
- `ts set` array append/remove: `linker.args+=-Wl,--gc-sections` / `linker.args-=-static`
- `ts set/unset` rewritten with `toml_edit` - supports all fields (including arrays: `rustc.build_std`, `linker.args`, `cargo.unstable`, `rustc.flags`), preserves comments/formatting
- `ts unset` command added - removes fields from tspecs
- `ts backup`, `ts restore`, `ts new -f` now use raw file copy (byte-for-byte, preserves comments)
- Rename `--all` to `--workspace` (match cargo convention) [16]
- In-place `set`, add `backup` and `restore` subcommands [15]
- Add `cargo.target_dir` spec field for per-spec target directories [12],[14]
- Convert TsCommands to Execute pattern [13]
- Refactor to Execute trait with execute_cargo_subcommand() helper [10]
- Add `tspec install --path <path>` command [9]
- Add `tspec clean` command - wrap `cargo clean` for completeness with build/run/test
- Set up CI/CD [5]
- Rename `tspec` subcommand to `ts` (avoid `tspec tspec list`) [3]
- Update README for standalone usage [6]
- Add POP (Plain Old Package) support [1]
- Rename package and binary from `xt` to `tspec` [7]
- Verify tests pass in isolation [8]
- Migrate git history from rlibc-x using subtree split [2]

[1]: tspec-design.md#20260202---augment-vs-replace-cargo
[2]: chores-1.md#20260202---initial-setup-after-migration
[3]: chores-1.md#rename-tspec-subcommand-to-ts
[5]: chores-1.md#cicd
[6]: chores-1.md#update-readme
[7]: chores-1.md#rename-package-and-binary
[8]: chores-1.md#test-in-isolation
[9]: chores-1.md#add-tspec-install---path
[10]: chores-1.md#cargopassthrough-trait-for-wrapper-commands
[12]: chores-1.md#per-spec-target-directories
[13]: chores-2.md#convert-commandsts-to-use-execute-trait
[14]: chores-2.md#20260206---add-cargotarget_dir-spec-field
[15]: chores-2.md#20260207---in-place-set-add-backup-and-restore-subcommands
[16]: chores-2.md#20260208---rename---all-to---workspace
[17]: chores-3.md#20260211---orthogonal-ts-setaddremove-with-separate-key-and-value-args
[18]: chores-4.md#20260211---fix-compare-optional--p-and-glob--t-handling
[19]: chores-4.md#20260212---always-include-cargo---release-baseline-in-compare
[20]: chores-4.md#20260212---detect-and-remove-stale-tspec-generated-buildrs
[21]: chores-4.md#20260212---design-tspec-build-library-for-linkerargs
[22]: chores-4.md#20260214---add-compare---workspace-for-all-packages-mode
[24]: chores-4.md#20260215---remove-rustcpanic-duplicate-of-global-panic
