# Chores-2

Continued maintenance tasks, following on from chores-1.

## 20260206 - Ts Execute Conversion

### Convert `Commands::Ts` to use Execute trait

The `Ts` variant in `Commands` is the only command that doesn't use the `Execute` trait. All other commands (Build, Run, Test, Clean, Clippy, Fmt, Compare, Version, Install) follow the pattern where a command struct implements `Execute` and main.rs calls `cmd.execute(&project_root)`. The `Ts` subcommands instead destructure fields in main.rs and call free functions from `ts_cmd::*`.

**Current state in main.rs:**
```rust
Commands::Ts { command } => match command {
    TsCommands::List { package, all } => {
        ts_cmd::list_tspecs(package.as_deref(), all)?;
    }
    TsCommands::Show { package, all, tspec } => {
        ts_cmd::show_tspec(package.as_deref(), all, tspec.as_deref())?;
    }
    TsCommands::Hash { package, all, tspec } => {
        ts_cmd::hash_tspec(package.as_deref(), all, tspec.as_deref())?;
    }
    TsCommands::New { name, package, from } => {
        ts_cmd::new_tspec(package.as_deref(), &name, from.as_deref())?;
    }
    TsCommands::Set { assignment, package, tspec } => {
        let (key, value) = assignment.split_once('=')...;
        ts_cmd::set_value(package.as_deref(), key, value, tspec.as_deref())?;
    }
},
```

**Goal state in main.rs:**
```rust
Commands::Ts(cmd) => {
    cmd.execute(&find_project_root()?)?;
}
```

**Implementation steps:**

1. **Create `src/cmd/ts.rs`** with a `TsCmd` struct that wraps `TsCommands`:
   ```rust
   #[derive(Args)]
   pub struct TsCmd {
       #[command(subcommand)]
       command: TsCommands,
   }

   impl Execute for TsCmd {
       fn execute(&self, project_root: &Path) -> Result<ExitCode> {
           match &self.command {
               TsCommands::List { package, all } => {
                   ts_cmd::list_tspecs(package.as_deref(), *all)?;
               }
               // ... other variants
           }
           Ok(ExitCode::SUCCESS)
       }
   }
   ```

2. **Move `TsCommands` enum** from `cli.rs` to `src/cmd/ts.rs` (or keep in cli.rs and import — either works, but co-locating with the Execute impl is cleaner).

3. **Update `cli.rs`** to use `Ts(TsCmd)` tuple variant instead of `Ts { command: TsCommands }` struct variant:
   ```rust
   // Before
   Ts {
       #[command(subcommand)]
       command: TsCommands,
   },
   // After
   Ts(TsCmd),
   ```

4. **Update `src/cmd/mod.rs`** to add `mod ts;` and `pub use ts::TsCmd;`.

5. **Update main.rs** to replace the 30-line `Ts` match arm with the one-liner.

6. **Consider whether `ts_cmd/` functions need `project_root`** — currently they call `find_project_root()` internally or use `current_dir()`. The Execute trait passes `project_root`, so the ts_cmd functions could be updated to accept it instead of finding it themselves. This is optional but would make them consistent.

**Files affected:**
- `src/cmd/ts.rs` (new)
- `src/cmd/mod.rs` (add module + re-export)
- `src/cli.rs` (change `Ts` variant, possibly move `TsCommands`)
- `src/main.rs` (simplify match arm)
- `src/ts_cmd/*.rs` (optional: accept `project_root` parameter)

**Status:** Todo
