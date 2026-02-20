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

**dev1:** Add `--test <NAME>` and trailing args (`-- <args>`) to `tspec test` so we can
run integration tests via tspec itself: `tspec test -p tspec --test integration_test -- --ignored`

**dev2:** POP fixture in `tests/fixtures/pop/` with copy-to-tmpdir helper and `#[ignore]` integration tests.

**dev3:** POP+WS fixture in `tests/fixtures/pop-ws/` with workspace discovery and cross-package tests.

**dev4:** POWS fixture in `tests/fixtures/pows/` with all-packages mode tests.

Full plan: `.claude/plans/enumerated-booping-curry.md`
