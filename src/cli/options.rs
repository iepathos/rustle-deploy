use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Main rustle-deploy CLI interface with Ansible compatibility
#[derive(Parser)]
#[command(name = "rustle-deploy")]
#[command(about = "Zero-infrastructure Ansible replacement with binary deployment optimization")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct RustleDeployCli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Playbook file (Ansible-compatible)
    pub playbook: Option<PathBuf>,

    /// Inventory file with target host information
    #[arg(short, long)]
    pub inventory: Option<PathBuf>,

    /// Optimization mode for deployment strategy
    #[arg(long, value_enum, default_value = "auto")]
    pub optimization: OptimizationMode,

    /// Force binary deployment (skip SSH fallback)
    #[arg(long)]
    pub force_binary: bool,

    /// Force SSH deployment (skip binary optimization)
    #[arg(long)]
    pub force_ssh: bool,

    /// Enable verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbosity: u8,

    /// Show what would be deployed without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Enable binary caching for faster rebuilds
    #[arg(long, default_value = "true")]
    pub cache_binaries: bool,

    /// Enable parallel compilation
    #[arg(long, default_value = "true")]
    pub parallel_compilation: bool,

    /// Cache directory for compilation artifacts
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,

    /// Target architecture override (auto-detect from inventory)
    #[arg(long)]
    pub target: Option<String>,

    /// Deployment timeout per host (seconds)
    #[arg(long, default_value = "300")]
    pub timeout: u64,

    /// Maximum parallel compilation jobs
    #[arg(long)]
    pub parallel_jobs: Option<usize>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check cross-compilation capabilities and setup
    CheckCapabilities {
        /// Show detailed capability information
        #[arg(long)]
        verbose: bool,
        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    
    /// Install missing dependencies for zero-infrastructure compilation
    Setup {
        /// Install Zig if missing
        #[arg(long)]
        install_zig: bool,
        /// Install cargo-zigbuild if missing
        #[arg(long)]
        install_zigbuild: bool,
        /// Install all recommended components
        #[arg(long)]
        install_all: bool,
    },

    /// Compile playbook to optimized binaries
    Compile {
        /// Playbook file
        playbook: PathBuf,
        /// Inventory file
        #[arg(short, long)]
        inventory: PathBuf,
        /// Output directory for binaries
        #[arg(short, long, default_value = "./target")]
        output: PathBuf,
    },

    /// Deploy compiled binaries to target hosts
    Deploy {
        /// Deployment plan or playbook file
        plan: PathBuf,
        /// Inventory file
        #[arg(short, long)]
        inventory: PathBuf,
    },

    /// Show deployment optimization analysis
    Analyze {
        /// Playbook file
        playbook: PathBuf,
        /// Inventory file
        #[arg(short, long)]
        inventory: PathBuf,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OptimizationMode {
    /// Automatic optimization decisions based on analysis
    Auto,
    /// Prefer binary deployment when possible
    Aggressive,
    /// Prefer SSH with selective binary optimization
    Conservative,
    /// SSH deployment only
    Off,
}

#[derive(Debug, Clone)]
pub struct DeployOptions {
    pub optimization_mode: OptimizationMode,
    pub force_binary: bool,
    pub force_ssh: bool,
    pub verbosity: u8,
    pub dry_run: bool,
    pub cache_binaries: bool,
    pub parallel_compilation: bool,
    pub timeout: u64,
    pub parallel_jobs: Option<usize>,
}

impl From<&RustleDeployCli> for DeployOptions {
    fn from(cli: &RustleDeployCli) -> Self {
        Self {
            optimization_mode: cli.optimization.clone(),
            force_binary: cli.force_binary,
            force_ssh: cli.force_ssh,
            verbosity: cli.verbosity,
            dry_run: cli.dry_run,
            cache_binaries: cli.cache_binaries,
            parallel_compilation: cli.parallel_compilation,
            timeout: cli.timeout,
            parallel_jobs: cli.parallel_jobs,
        }
    }
}