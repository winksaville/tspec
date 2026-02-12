# Chores-4

## 20260211 - Fix compare: optional `-p` and glob `-t` handling

### Context

The `compare` command has two issues:

1. **`-p` is required but should be optional.** Build, Run, and Test all default to the current directory package when `-p` is omitted. Compare requires it, which is inconsistent and unnecessary for POPs.

2. **`-t` with shell-expanded globs fails.** The `-t` flag uses clap's `Append` action, which captures only one value per `-t` flag. When the shell expands an unquoted glob like `tspec compare -t *.ts.toml` into `tspec compare -t file1.ts.toml file2.ts.toml`, clap rejects the second file as an unexpected argument.

### Plan

**Step 1: `src/cmd/compare.rs` — Make `-p` optional, fix `-t`**

- Change `package: String` to `Option<String>` with `current_package_name()` fallback
- Change `-t` from `action = Append` to `num_args = 1..` so shell-expanded globs work
- Use `resolve_package_dir()` + `get_package_name()` in `execute()`

**Step 2: Add tests**

- CLI parse tests for `CompareCmd`: optional `-p`, `-t` with multiple values, no `-t` defaults
- `find_tspecs` test for multi-dot filenames (`tspec.musl.ts.toml` matching `tspec*.ts.toml`)

### Result

Done. `-p` is now optional (defaults to cwd package), `-t` accepts shell-expanded globs via `num_args = 1..`. Removed broken spec files (dyn-opt, static-opt). 8 new tests added.

### References

- todo.md items: "-p shouldn't be needed for `ts compare` if in a POP" and "for build, run ... a -t should support glob like in compare"

## 20260212 - Always include cargo --release baseline in compare

### Context

`tspec compare` only compares builds using tspec files. If no tspec files exist, it errors out. We want a plain `cargo build --release` result always included as a baseline reference point — even with zero tspec files.

### Problems

- `build_package(pkg_name, None, release)` auto-discovers default `tspec.ts.toml` if it exists — no way to force a plain build
- `find_tspecs()` errors when no tspec files match — compare can't run without specs
- `compare_specs()` only iterates spec paths, no baseline concept

### Plan

1. Add `build_package_plain()` in `cargo_build.rs` — always does plain cargo build, skips spec lookup
2. Add `build_baseline()` helper in `compare.rs`, modify `compare_specs()` to build cargo --release first
3. Allow empty tspec list in `cmd/compare.rs` — default pattern returns empty vec instead of erroring

### Result

(pending)
