# Todo

## In Progress

- Orthogonal `ts set`/`add`/`remove` with separate key and value args [17]

## Todo

- Investigate `-static` vs `dynamic-linking=false` size difference (partially explained); note: glibc + `-static` segfaults (glibc not designed for static linking), consider musl for static builds [11]
- Improve `classify_crate` - using name alone is brittle [4]
- -p shouldn't be needed for `ts compare` if in a POP
- for build, run ... a -t should suppor glob like in compare
 

## Done

See older [done.md](done.md)

- `ts set` array append/remove: `linker.args+=-Wl,--gc-sections` / `linker.args-=-static`
- `ts set/unset` rewritten with `toml_edit` - supports all fields (including arrays: `rustc.build_std`, `linker.args`, `cargo.unstable`, `rustc.flags`), preserves comments/formatting
- `ts unset` command added - removes fields from tspecs
- `ts backup`, `ts restore`, `ts new -f` now use raw file copy (byte-for-byte, preserves comments)
- Rename `--all` to `--workspace` (match cargo convention) [16]
- In-place `set`, add `backup` and `restore` subcommands [15]
- Add `cargo.target_dir` spec field for per-spec target directories [12],[14]

[4]: chores-1.md#improve-classify_crate
[11]: chores-1.md#investigate--static-vs-dynamic-linkingfalse-size-difference
[12]: chores-1.md#per-spec-target-directories
[14]: chores-2.md#20260206---add-cargotarget_dir-spec-field
[15]: chores-2.md#20260207---in-place-set-add-backup-and-restore-subcommands
[16]: chores-2.md#20260208---rename---all-to---workspace
[17]: chores-3.md#20260211---orthogonal-ts-setaddremove-with-separate-key-and-value-args
