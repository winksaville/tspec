# Chores 6

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
- Removed `test_members()` method
- Replaced `kind: PackageKind` with `is_build_tool: bool` on `PackageMember`
- Added `is_build_tool_name()` — checks tspec/xt/xtask
- Updated `discover()`, `buildable_members()`, `runnable_members()`
- Replaced 11 classify tests with 2 `is_build_tool_name` tests

**`src/all.rs`:**
- Removed `PackageKind`, `PermissionsExt` imports
- Removed `PackageKind::Test` skip and entire 100-line
  test-binary-discovery block from `test_all()`
- `test_all()` now just iterates `buildable_members()` + `test_package()`

### Behavioral changes

- `runnable_members()` returns any non-build-tool with a binary
  (was: only `App` kind)
- Test packages formerly getting special binary-discovery handling
  now get normal `cargo test`
- Neither change affects this repo

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
