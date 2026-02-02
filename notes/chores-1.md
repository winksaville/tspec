# Chores-1

General maintenance tasks and considerations for the project.

## 20260202 - Initial Setup After Migration

Migrated xt from rlibc-x to standalone tspec repo using `git subtree split`.

### Chores

1. **POP Support** - Make xt work with Plain Old Packages (single Cargo.toml without workspace). Currently errors with "could not find workspace root". [design](tspec-design.md#20260202---augment-vs-replace-cargo)

2. **Rename binary** - Consider renaming from `xt` to `tspec` to match repo name. Affects CLI invocation (`cargo xt` vs `cargo tspec` or standalone `tspec`).

3. **Update README** - Remove rlibc-x specific references, update for standalone usage.

4. **Package name** - Cargo.toml still has `name = "xt"`. Consider changing to `tspec`.

5. **Test in isolation** - Verify all tests pass without rlibc-x workspace context.

6. **CI/CD** - Set up GitHub Actions or similar for automated testing.
