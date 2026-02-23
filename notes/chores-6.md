# Chores 6

## 20260220 - Parse cargo test output for counts and filtering

### Problem

`tspec test` streams cargo output live but has no insight into what
happened. The summary shows only `[PASS]`/`[FAIL]` per package with
no test counts. Worse, 0 tests running reports as success — a filter
typo silently passes. And `--test <target>` emits noisy "running 0
tests" blocks for non-matching targets within a package.

### Solution

Tee stdout line-by-line (live output + selective capture), parse
cargo's `test result:` lines for counts, and surface them:

- Per-package counts in summary: `[PASS]  263 passed`
- Aggregate footer: `Test: 2 packages, 279 passed, 0 failed`
- Fail (exit 1) when 0 tests ran in single-package mode
- Filter "running 0 tests" noise

Infrastructure in a reusable `tee.rs` module so future commands
(build warnings, benchmarks) can use the same pattern.

### Plan

- **dev1:** tee.rs + TestResult + parse_test_result_line + wire into
  run_cargo for test mode. No visible behavior change.
- **dev2:** Summary counts, 0-test failure, noise filtering.
- **dev3:** Fail fixtures + failure-path tests.

### Decision: fail fixtures for testing failure paths

Considered external repos vs in-tree fixtures. In-tree is better
for normal development. Fail fixtures (`pop-fail`, `pows-fail`)
live in `tests/fixtures/`, workspace-excluded, with actual failing
tests baked in. Integration tests using them are `#[ignore]`'d —
skipped during normal `tspec test`, run with `-- --ignored`.

Manual testing: `cd tests/fixtures/pop-fail && tspec test`.

Also add unit tests for `print_test_summary` with synthetic
OpResult data (pass/fail/mixed with counts, verify exit codes).

### Result

- `tee.rs` — reusable tee_stdout(cmd, filter, suppress) utility
- `cargo_build.rs` — run_cargo returns raw matched lines for Test
  mode; noise suppression for "running 0 tests" blocks
- `cmd/test.rs` — TestResult, parse_test_result_line(),
  parse_test_results(); 0-test guard for single-package mode
- `all.rs` — OpResult.test_counts, per-package counts in summary,
  aggregate footer
- 0-test guard only in single-package mode (workspace fixtures
  legitimately have 0 tests)

## 20260220 - Remove classify_package and PackageKind

### Context

`classify_package()` hardcoded directory conventions (`apps/`, `libs/`,
`tools/`, `*/tests`) specific to one workspace layout. tspec shouldn't
impose layout opinions — cargo metadata already provides `has_binary`,
and the only real behavioral need is excluding build tools (tspec/xt/xtask)
from batch operations.

### Changes

**`src/workspace.rs`:**
- Removed `PackageKind` enum (5 variants) and `classify_package()` fn
- Removed `test_members()` method and `is_build_tool` field
- `PackageMember` now has only `name`, `path`, `has_binary`
- `buildable_members()` returns all members (no exclusions)
- `runnable_members()` returns all members with binaries
- Replaced 11 classify tests with 1 `discover_works` test

**`src/all.rs`:**
- Removed `PackageKind`, `PermissionsExt` imports
- Removed `PackageKind::Test` skip and entire 100-line
  test-binary-discovery block from `test_all()`
- `test_all()` now just iterates `buildable_members()` + `test_package()`

**`tests/integration_test.rs`:**
- Removed `#[ignore]` from all 16 integration tests — they run
  by default now since they're fast and anyone running `tspec test`
  has the full toolchain

### Rationale for removing is_build_tool

The `xt`/`xtask` exclusions were vestigial from the rlibc-x workspace.
Excluding `tspec` from its own `tspec test` was counterproductive —
`tspec test` only showed tspec-build in the summary. No workspace
layout opinions should be baked into tspec.

### Behavioral changes

- `tspec test` (all-packages mode) now includes all workspace members
- `runnable_members()` returns any member with a binary (no exclusions)
- Integration tests run by default (no `-- --ignored` needed)

## 20260223 - Add --manifest-path flag with --mp alias

### Problem

tspec currently requires `cd`ing into a project to operate on it.
Adding `--manifest-path` (with `visible_alias = "mp"`) lets users run
`tspec build --mp /path/to/project` from anywhere, matching cargo's
own `--manifest-path` convention but with a shorter alias.

### Approach: thread project_root through internal APIs

The core insight: `project_root` is already computed in `main.rs` and
passed to every command's `execute()`. But three helpers re-derive it
from cwd:

- `resolve_package_arg()` calls `find_project_root()` internally
- `current_package_name()` uses `std::env::current_dir()`
- `WorkspaceInfo::discover()` runs `cargo metadata` from cwd

Fix: refactor these three to accept `project_root` as a parameter,
then wire `--manifest-path` into `main.rs` to override
`find_project_root()`.

### Plan (multi-step)

- **dev1:** Refactor `resolve_package_arg()`, `current_package_name()`,
  and `WorkspaceInfo::discover()` to accept `project_root`. Update all
  callers. No behavior change.
- **dev2:** Add `--manifest-path` / `--mp` global flag to CLI.
  Add `resolve_manifest_path()` to `find_paths.rs`. Wire into
  `main.rs` to override `find_project_root()`.

### Status

In progress.

## 20260219 - Test infrastructure: fixture workspaces and test args

### Context

tspec has 255 unit tests but zero integration tests that actually build real packages.
The top todo item is "Add a permanent test workspace for integration testing."

Three project structures need test coverage:
1. **POP** — Plain Old Package (single Cargo.toml, no workspace)
2. **POP+WS** — POP with embedded workspace (like tspec itself)
3. **POWS** — Pure Old Workspace (workspace root, no package)

### Plan

**dev1:** Add test selection flags to `tspec test`:
- `--test <TARGET>` (repeatable) — select specific test targets
- `--names/-n <NAME>...` — filter tests by name (OR-matched substrings via test harness)
- `--list/-l` — list test targets and functions (grouped with counts)
- `-- <ARGS>` — pass trailing args to test binary (--ignored, --exact, etc.)
All flow through `CargoFlags.extra_args` — no new parameters to `test_package()`/`run_cargo()`.

**dev2:** POP fixture in `tests/fixtures/pop/` with copy-to-tmpdir helper and `#[ignore]` integration tests.

**dev3:** POP+WS fixture in `tests/fixtures/pop-ws/` with workspace discovery and cross-package tests.

**dev4:** POWS fixture in `tests/fixtures/pows/` with all-packages mode tests.

### Status (20260220)

**Branch:** `feat-test-fixtures` off `main` (at `a98ebad`)

**Completed:**
- dev0: `534ce4b` — marker commit, documented -dev0 convention in CLAUDE.md + README.md
- dev1: `28baac3` — test selection flags (--test, --names, --list, --target-names, trailing args)
  - Also added: `extra_args: Vec<String>` to `CargoFlags` in types.rs
  - Also added: workspace-mode --test auto-filters to packages that have the target
  - 268 unit tests, all passing. Installed and verified.
- dev2: `f85a1cb` — POP fixture + integration tests
  - Created `tests/fixtures/pop/` with Cargo.toml, src/main.rs, tspec.ts.toml
  - Added `exclude = ["tests/fixtures"]` to root Cargo.toml `[workspace]`
  - Created `tests/fixture.rs` helper: `copy_fixture(name) -> (TempDir, PathBuf)`
  - Created `tests/integration_test.rs` with 4 `#[ignore]` tests
  - Bug fix: `find_project_root()` now respects workspace `exclude` list
  - Added `is_excluded_from_workspace()` function + 4 unit tests
- dev3: `5c24d4b` — POP+WS fixture
  - Created `tests/fixtures/pop-ws/` with root binary + mylib member
  - Added 6 integration tests (spec load, cargo build, tspec build root/member/all, build from member dir)
  - Bug fix: `classify_package()` now uses paths relative to workspace root
  - Added unit test for path-under-tests-dir classification
- dev4: POWS fixture
  - Created `tests/fixtures/pows/` with workspace-only root (no [package]), pows-app binary, mylib library
  - Added 6 integration tests (cargo build, tspec build all/member/from-member-dir, tspec test all, dot-resolves-to-all)
  - Fix: directory name must match package name for `find_package_dir()` resolution (pows-app/ not app/)
  - All 16 integration tests passing, 268 unit tests passing

**Remaining:** release (v0.15.0)

### Future: --manifest-path / --path flag

Add a way to run tspec against a project without cd'ing into it. Cargo uses `--manifest-path <path/to/Cargo.toml>` (not `--path` — unclear why). We could follow cargo's convention (`--manifest-path`) or use `--path <dir>` for convenience. Would simplify integration tests and general usability. Not blocking dev2-dev4.

### Documentation TODO

Where and how to document the test fixtures (decide before release):
- `README.md` — new `## Testing tspec` section documenting when/how to run integration tests
- `tests/README.md` — purpose of the tests directory, overview of unit vs integration tests
- `tests/fixtures/README.md` — purpose of fixtures, how copy_fixture() works
- `tests/fixtures/pop/README.md` (and each fixture) — purpose of that specific fixture

Open question: should the per-fixture READMEs be the "main" docs that README.md links to, or should README.md be self-contained with the READMEs as secondary?

Full plan: `.claude/plans/enumerated-booping-curry.md`
