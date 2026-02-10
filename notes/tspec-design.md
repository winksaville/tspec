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
  calling `build_crate()` (misnamed — actually builds a package) for each
- Single package: user passes `-p <name>`, `build_crate()` runs `cargo build -p <name>`
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


