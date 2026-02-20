# Chores 6

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
- dev1: `9276af0` — test selection flags (--test, --names, --list, --target-names, trailing args)
  - Also added: `extra_args: Vec<String>` to `CargoFlags` in types.rs
  - Also added: workspace-mode --test auto-filters to packages that have the target
  - 268 unit tests, all passing. Installed and verified.

**Next: dev2** — POP fixture + integration tests
- Create `tests/fixtures/pop/` with minimal Cargo.toml, src/main.rs, tspec.ts.toml
- Add `exclude = ["tests/fixtures"]` to root Cargo.toml `[workspace]`
- Add `tests/fixtures/*/target/` to .gitignore
- Create `tests/fixture.rs` helper: `copy_fixture(name) -> (TempDir, PathBuf)`
- Create `tests/integration_test.rs` with `#[ignore]` tests:
  - cargo build succeeds on fixture copy
  - spec loading returns expected values
  - `tspec build .` produces binary
  - `tspec compare .` succeeds
- Run via: `tspec test --test integration_test -- --ignored`
- Bump version to v0.15.0-dev2

**Remaining:** dev3 (POP+WS fixture), dev4 (POWS fixture), release (v0.15.0)

Full plan: `.claude/plans/enumerated-booping-curry.md`
