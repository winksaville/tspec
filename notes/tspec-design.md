# tspec Design

## 20260202 - Augment vs Replace Cargo

**Question:** Should tspec augment or replace cargo?

**Decision:** Augment cargo (Option 1)

### Context

Currently `xt` is designed to *augment* cargo:
- It wraps `cargo build/test/run` with extra configuration (tspec files)
- It generates temporary `build.rs` files for scoped linker flags
- It requires an existing workspace structure

Running `xt build` outside a workspace fails with "could not find workspace root".

### Options Considered

1. **Augment cargo (chosen)** - Stay as a build orchestrator that sits on top of cargo. Users still use cargo directly for simple builds, use tspec when they need spec-driven configuration.

2. **Replace cargo** - Become a standalone build tool that doesn't require cargo's workspace structure. Much larger scope - would need to handle dependency resolution, compilation, etc.

3. **Hybrid** - Work standalone for simple cases (single crate) but leverage workspace features when available.

### Rationale

Option 1 (augment) is the practical choice. Replacing cargo is a massive undertaking. The "workspace required" limitation can be relaxed by treating a single `Cargo.toml` (Plain Old Package / POP) as a trivial workspace.

### First Task

Make tspec work with both workspaces and POPs (Plain Old Packages - single crate projects without a workspace).
