use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tspec", version, about = "Translation spec based build system")]
#[command(before_help = concat!("tspec ", env!("CARGO_PKG_VERSION")))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build package(s) with a translation spec
    Build {
        /// Package to build (defaults to current directory or all packages)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Build all packages (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Translation spec to use (defaults to package's tspec file)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
        /// Release build
        #[arg(short, long)]
        release: bool,
        /// Strip symbols from binary after build
        #[arg(short, long)]
        strip: bool,
        /// Stop on first failure (for all-packages mode)
        #[arg(short, long)]
        fail_fast: bool,
    },
    /// Build and run package(s) with a translation spec
    Run {
        /// Package to run (defaults to current directory or all apps)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Run all apps (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Translation spec to use (defaults to package's tspec file)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
        /// Release build
        #[arg(short, long)]
        release: bool,
        /// Strip symbols from binary before running
        #[arg(short, long)]
        strip: bool,
        /// Arguments to pass to the binary (after --)
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Test package(s) with a translation spec
    Test {
        /// Package to test (defaults to current directory or all packages)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Test all packages (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Translation spec to use (defaults to package's tspec file)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
        /// Release build
        #[arg(short, long)]
        release: bool,
        /// Stop on first failure
        #[arg(short, long)]
        fail_fast: bool,
    },
    /// Clean build artifacts
    Clean {
        /// Package to clean (defaults to entire workspace/project)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Only clean release artifacts
        #[arg(short, long)]
        release: bool,
    },
    /// Compare specs for a package (size only)
    Compare {
        /// Package to compare (required)
        #[arg(short = 'p', long = "package")]
        package: String,
        /// Spec file(s) or glob pattern(s) (defaults to tspec* pattern)
        #[arg(short = 't', long = "tspec", action = clap::ArgAction::Append)]
        tspec: Vec<String>,
        /// Release build
        #[arg(short, long)]
        release: bool,
        /// Strip symbols before comparing sizes
        #[arg(short, long)]
        strip: bool,
    },
    /// Manage package compatibility with specs
    Compat {
        /// Package name
        #[arg(short = 'p', long = "package")]
        package: String,
        /// Spec to add to compat list (omit to show current state)
        spec: Option<String>,
    },
    /// Mark a spec as incompatible with a package
    Incompat {
        /// Package name
        #[arg(short = 'p', long = "package")]
        package: String,
        /// Spec to add to incompat list
        spec: String,
    },
    /// Manage translation specs
    Ts {
        #[command(subcommand)]
        command: TsCommands,
    },
    /// Print version information
    Version,
}

#[derive(Subcommand)]
pub enum TsCommands {
    /// List tspec files in workspace or for a specific package
    List {
        /// Package to list specs for (defaults to current directory or all packages)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// List all packages (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
    },
    /// Show a tspec's contents
    Show {
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Show all packages (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Tspec name (defaults to all tspec files)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Show the content hash of a tspec
    Hash {
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Hash all packages (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Tspec name (defaults to package's tspec file)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Create a new tspec file
    New {
        /// Name for the new tspec (default: "tspec")
        #[arg(default_value = "tspec")]
        name: String,
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Copy from existing tspec (package/spec or just spec name in same package)
        #[arg(short = 'f', long = "from")]
        from: Option<String>,
    },
    /// Set a scalar value in a tspec (creates versioned copy)
    Set {
        /// Key=value pair (e.g., "strip=symbols", "panic=abort", "rustc.lto=true")
        assignment: String,
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Tspec to modify (defaults to package's tspec.ts.toml)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
}
