# Todo

## In Progress

## Todo

- Add `cargo.target_dir` spec field for per-spec target directories [12]
- Investigate `-static` vs `dynamic-linking=false` size difference (partially explained) [11]
- Improve `classify_crate` - using name alone is brittle [4]
- Investigate converting TsCommands to Execute pattern [13]

## Done

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
[4]: chores-1.md#improve-classify_crate
[5]: chores-1.md#cicd
[6]: chores-1.md#update-readme
[7]: chores-1.md#rename-package-and-binary
[8]: chores-1.md#test-in-isolation
[9]: chores-1.md#add-tspec-install---path
[10]: chores-1.md#cargopassthrough-trait-for-wrapper-commands
[11]: chores-1.md#investigate--static-vs-dynamic-linkingfalse-size-difference
[12]: chores-1.md#per-spec-target-directories
[13]: cli.rs - TsCommands enum uses inline struct variants, different from Execute pattern
