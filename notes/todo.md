# Todo

## In Progress

## Todo

- Investigate `-static` vs `dynamic-linking=false` size difference (partially explained); note: glibc + `-static` segfaults (glibc not designed for static linking), consider musl for static builds [11]
- Improve `classify_crate` - using name alone is brittle [4]
- -p shouldn't be needed for `ts compare` if in a POP
- for build, run ... a -t should suppor glob like in compare
- add a `ts unset` or `ts remove`?
- how to add/remove from a array/list such as linker.args, rustc.flags and many others
- Some fields in README.md don't seem to be working not just these
-- wink@fwlaptop 26-02-09T06:26:58.397Z:~/data/prgs/rust/rlibc-x/apps/hw-x2 (main)
-- $ tspec ts set -t tspec-opt-2.ts.toml rustc.build_std="abc"
-- Error: unknown key: rustc.build_std
-- wink@fwlaptop 26-02-09T06:27:22.688Z:~/data/prgs/rust/rlibc-x/apps/hw-x2 (main)
-- $ tspec ts set -t tspec-opt-2.ts.toml rustc.build_std=["abc"]
-- Error: unknown key: rustc.build_std
-- wink@fwlaptop 26-02-09T06:27:33.476Z:~/data/prgs/rust/rlibc-x/apps/hw-x2 (main)
-- $ tspec ts set -t tspec-opt-2.ts.toml linker.args="abc"
-- Error: unknown key: linker.args
-- wink@fwlaptop 26-02-09T06:27:59.220Z:~/data/prgs/rust/rlibc-x/apps/hw-x2 (main)
-- $ tspec ts set -t tspec-opt-2.ts.toml args="abc"
-- Error: unknown key: args
-- wink@fwlaptop 26-02-09T06:28:15.285Z:~/data/prgs/rust/rlibc-x/apps/hw-x2 (main)


## Done

See older [done.md](done.md)

- Rename `--all` to `--workspace` (match cargo convention) [16]
- In-place `set`, add `backup` and `restore` subcommands [15]
- Add `cargo.target_dir` spec field for per-spec target directories [12],[14]

[4]: chores-1.md#improve-classify_crate
[11]: chores-1.md#investigate--static-vs-dynamic-linkingfalse-size-difference
[12]: chores-1.md#per-spec-target-directories
[14]: chores-2.md#20260206---add-cargotarget_dir-spec-field
[15]: chores-2.md#20260207---in-place-set-add-backup-and-restore-subcommands
[16]: chores-2.md#20260208---rename---all-to---workspace
