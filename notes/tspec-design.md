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

Make tspec work with both workspaces and POPs (Plain Old Packages - single crate projects without a workspace).(**done**)

## 20260209 - Crate, Package, Cargo.toml

**Conclusion, tspec provides a mechanism to describe how crates are compiled and linked.**

### Rust Compilation unit

General question about rust compilation units[[1]]

[1]:https://claude.ai/share/c621b72e-bebe-494a-b9f8-639cbaf10f07

Crate is the compilation unit of rust.
* Binary crate: produces an executable
* Library crate: produces a library

Q: a binary crate uses multiple library crates can each library crate have a different tspec?
* yes
* but the final link step needs compatible link settings
  * panic setting must be compatible
  * abi affecting options must be compatible
  * if not the linker should/will produce errors

Q: does rustc or cargo enforce the definition of a crate?
* rustc
* cargo is just an orchestrator

Q: So I cannot give rustc a file to compile?
* No, rustc is always given a single file as the root of the crate.
  see formal definition of crate in the rust reference below [[2]]

  
Q: If foo.rs is only `fn f() -> i8 { 1 }` it is a crate and can have a tspec.ts.toml
associated with it to control its compilation?
* yes, but as defined it would be private, it should be `pub fn f() -> i8 { 1 }`

Q: sym tables (me free thinking wild possibilities here)
* We may want capabilities to control symbol visibility/availability in tspec.
  Symbols are tricky and if we did this it probably would use glob/regex patterns
  as matching individual symbols would be fragile?
  In any case we wouldn't want to disallow either case.


### Package, Crates and Modules

[book](https://doc.rust-lang.org/stable/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html):
Rust has a number of features that allow you to manage your
code’s organization, including which details are exposed,
which details are private, and what names are in each scope
in your programs. These features, sometimes collectively
referred to as the module system, include:

* Packages: A Cargo feature that lets you build, test, and share crates
* Crates: A tree of modules that produces a library or executable
* Modules and use: Let you control the organization, scope, and privacy of paths
* Paths: A way of naming an item, such as a struct, function, or module

#### Package

[book](https://doc.rust-lang.org/stable/book/ch07-01-packages-and-crates.html)
A package is a bundle of one or more crates that provides a set of functionality. A package contains a Cargo.toml file that describes how to build those crates.

#### Crate
A rust crate, as stated by [[2]] says; "The Rust compiler is always invoked with a single source file as input, and always produces a single output crate." Thus, a crate is defined by the .rs file passed to the compiler and all of its dependencies.

[2]:https://doc.rust-lang.org/reference/crates-and-source-files.html

Crates are imported using `extern crate` [3],
"An extern crate declaration specifies a dependency on an external crate."

In [The Rust Reference](https://doc.rust-lang.org/reference) the term "package" is only found in extern-crates[3] and specifically [here](https://doc.rust-lang.org/reference/items/extern-crates.html?search=#r-items.extern-crate.name-restrictions)

So from a language PoV there are crates but no packages. A package is a Cargo concept, not a Rust language concept.
[3](https://doc.rust-lang.org/reference/items/extern-crates.html)

#### Modules [ref](https://doc.rust-lang.org/stable/reference/items/modules.html), [book](https://doc.rust-lang.org/stable/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)

* [intro](https://doc.rust-lang.org/stable/reference/items/modules.html#r-items.mod.intro)
  A module is zero or more items.
* [def](https://doc.rust-lang.org/stable/reference/items/modules.html#r-items.mod.def)
  A module item is a module, surrounded in braces, named, and prefixed
  with the keyword mod. A module item introduces a new, named module
  into the tree of modules making up a crate.

#### Cargo terminology

* **Targets** — `[lib]`, `[[bin]]`, `[[example]]`, `[[test]]`, `[[bench]]`
  sections in Cargo.toml. These are the crates within a package.
* **Members** — packages listed in `[workspace] members` in the root Cargo.toml.

Within a package you have targets, within a workspace you have members.

### Package vs Crate granularity in tspec

**Question:** tspec currently operates at the package level, but conceptually
it describes crate compilation. How do we handle packages with multiple crates?

**Decision:** One package = one tspec. If you need per-crate control, split into
separate packages.

#### Current behavior

tspec operates at Cargo's package granularity:
- `build_all` discovers workspace packages via `cargo metadata` and iterates,
  calling `build_package()` for each
- Single package: user passes `-p <name>`, `build_package()` runs `cargo build -p <name>`
- `RUSTFLAGS` is set per invocation, applying to all crates within the package

A package can contain multiple crates (targets). For example, tspec itself has two:
- `src/lib.rs` → library crate (`--crate-type lib`)
- `src/main.rs` → binary crate (`--crate-type bin`)

Both get the same `RUSTFLAGS` from a single `cargo build -p tspec` invocation.

#### Options considered for per-crate tspecs

**1. Per-target tspecs within a package** — Use Cargo.toml `[lib]`/`[[bin]]`
target names to find `<target-name>.ts.toml`, then invoke `cargo rustc`
per target instead of `cargo build -p`.

Problems:
- `cargo rustc --lib -- <flags>` applies extra flags only to the named target,
  not its dependencies, but `RUSTFLAGS` (which tspec currently uses) hits everything
- Would need to stop using `RUSTFLAGS` and pass everything via `cargo rustc -- <flags>`
- Would need to manage build ordering (lib before bin) — becoming a mini build
  orchestrator on top of Cargo's orchestrator
- Fighting against Cargo's design

**2. Separate packages (chosen)** — If you need different compilation settings
for lib vs bin, split them into separate packages. Each gets its own directory,
`Cargo.toml`, and `tspec.ts.toml`. Workspace `members` ties them together.

This works today with zero changes to tspec. It's idiomatic Rust — this is
exactly why workspaces exist.

#### Rationale

Approach 2 keeps the model clean: **one package = one primary crate = one tspec**.
Approach 1 fights Cargo's design for marginal benefit. For the common case
(tspec itself: thin main.rs calling into lib.rs), there's no reason to compile
the lib and bin differently. When you truly need different settings, the Rust
answer is "make them separate packages."

## 20260216 - Build mechanisms and per-package scoping

### Current mechanisms (as of 0.10.9)

tspec passes settings to cargo/rustc through five mechanisms, each with different
scoping behavior:

| Mechanism | tspec source | How it's passed | Scope |
|---|---|---|---|
| Top-level fields (`panic`, `strip`) | Global fields in spec | RUSTFLAGS `-C` + cargo `-Z` | Package + all deps |
| `[cargo]` fields | `profile`, `target_triple`, `target_json`, `unstable`, `target_dir` | Cargo CLI args (`--release`, `--target`, `-Z`, `--target-dir`) | Package-scoped |
| `[cargo.config_key_value]` | Arbitrary key-value pairs | `cargo --config 'KEY=VALUE'` | Depends on key (see below) |
| `[rustc]` fields | `opt_level`, `lto`, `codegen_units`, `build_std`, `flags` | RUSTFLAGS `-C` | Package + all deps |
| `[linker]` fields | `args`, `version_script` | Generated `build.rs` or `tspec-build` library | Per-binary |

### The config_key_value scoping spectrum

`[cargo.config_key_value]` is the most flexible mechanism because the scoping is
determined by the key the user writes:

```toml
# Global — applies to the package and all its dependencies
[cargo.config_key_value]
"profile.release.opt-level" = "z"
"profile.release.codegen-units" = 1

# Per-package — applies ONLY to the named package
[cargo.config_key_value]
"profile.release.package.serde.opt-level" = 2
"profile.release.package.tspec.codegen-units" = 1
```

This maps to cargo's `[profile.release.package.<name>]` syntax. The supported
per-package profile fields are: `opt-level`, `codegen-units`, `overflow-checks`,
`debug`, `debug-assertions`, `strip`, `lto`. Notably excluded: `panic` (must be
consistent across the dependency graph) and `rpath` (whole-build concern).

### Per-package status

We now have the **mechanism** for per-package settings — users can manually write
`profile.release.package.<name>.<field>` keys in `config_key_value` today. What's
missing for full per-dependency tspec support:

1. **Dependency tspec discovery** — walking the dep tree to find each dependency's
   tspec file (known viable: `cargo package` preserves `*.ts.toml` files)
2. **Automatic key scoping** — translating a dependency's tspec fields into
   `--config 'profile.release.package.<dep>.<field>=...'` args
3. **Conflict detection** — ensuring `panic` strategy is consistent across the
   dependency graph

None of these are implemented yet. The manual path works for experimentation.

### The [rustc] field overlap

Several `[rustc]` fields duplicate what `[cargo.config_key_value]` can express:

| `[rustc]` field | Equivalent `config_key_value` key | Mechanism |
|---|---|---|
| `opt_level = "z"` | `"profile.release.opt-level" = "z"` | RUSTFLAGS vs `--config` |
| `lto = true` | `"profile.release.lto" = true` | RUSTFLAGS vs `--config` |
| `codegen_units = 1` | `"profile.release.codegen-units" = 1` | RUSTFLAGS vs `--config` |

Tested: both paths produce identical binary sizes (confirmed with tspec-build test
runner: 1,117,880 bytes both ways). Binaries are not bit-identical (~6.5% byte
difference) but the optimization effect is the same. See chores-5 for details.

The `[rustc]` fields that have NO `config_key_value` equivalent and must stay:
- `build_std` — cargo `-Z build-std`, not a profile setting
- `flags` — raw rustc flags with no profile equivalent

Migration of the overlapping fields is possible but not urgent. Both paths work.
The advantage of migrating: `config_key_value` supports per-package scoping via
`profile.*.package.<name>`, while RUSTFLAGS hits everything uniformly.

### Relationship to earlier design discussions

- [chores-4: Profile support and section scoping][25] — design analysis
- [chores-4: build.rs vs cargo --config][26] — confirmed `--config` is the right path
- [chores-4: config_key_value implementation][27] — the implementation
- [chores-5: RUSTFLAGS vs --config binary comparison][28] — same size, different bytes

[25]: chores-4.md#20260215---design-profile-support-and-tspec-section-scoping
[26]: chores-4.md#20260216---design-passing-tspec-fields-via-buildrs-vs-cargo---config
[27]: chores-4.md#20260216---implement-cargoconfig_key_value-support
[28]: chores-5.md#finding-rustflags-vs---config-produce-same-size-but-non-identical-binaries
