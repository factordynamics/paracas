//! paracas CLI - High-performance Dukascopy tick data downloader.

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod display;

use display::Format;

#[derive(Parser)]
#[command(name = "paracas")]
#[command(about = "High-performance Dukascopy tick data downloader", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Quiet mode (suppress progress output)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Hidden: Run as daemon with job ID (internal use only)
    #[arg(long, hide = true)]
    daemon_run: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Download tick data
    Download {
        /// Instrument identifier (e.g., eurusd, btcusd)
        instrument: String,

        /// Start date (YYYY-MM-DD). Defaults to instrument's earliest available data.
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD). Defaults to today.
        #[arg(short, long)]
        end: Option<String>,

        /// Output file path. Defaults to <instrument>.<format>
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "csv")]
        format: Format,

        /// OHLCV aggregation timeframe (omit for raw ticks)
        #[arg(short, long)]
        timeframe: Option<String>,

        /// Maximum concurrent downloads
        #[arg(long, default_value = "32")]
        concurrency: usize,

        /// Run in background as daemon
        #[arg(long)]
        background: bool,

        /// Skip confirmation prompt (for background mode)
        #[arg(long)]
        yes: bool,
    },

    /// List available instruments
    List {
        /// Filter by category (forex, crypto, index, stock, commodity, etf, bond)
        #[arg(short, long)]
        category: Option<String>,

        /// Search pattern
        #[arg(short, long)]
        search: Option<String>,
    },

    /// Show instrument details
    Info {
        /// Instrument identifier
        instrument: String,
    },

    /// Check background job status
    Status {
        /// Specific job ID to check
        job_id: Option<String>,

        /// Show only running jobs
        #[arg(long)]
        running: bool,

        /// Show all jobs (including completed)
        #[arg(long)]
        all: bool,

        /// Follow/watch mode (refresh every N seconds)
        #[arg(short, long)]
        follow: Option<u64>,

        /// Cancel a running job
        #[arg(long)]
        cancel: Option<String>,
    },

    /// Download all instruments (or filter by category)
    DownloadAll {
        /// Filter by category (forex, crypto, index, commodity)
        #[arg(short, long)]
        category: Option<String>,

        /// Start date (YYYY-MM-DD). Defaults to each instrument's earliest data.
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD). Defaults to today.
        #[arg(short, long)]
        end: Option<String>,

        /// Output directory. Files named <instrument>.<format>
        #[arg(short, long, default_value = ".")]
        output_dir: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "csv")]
        format: Format,

        /// OHLCV aggregation timeframe (omit for raw ticks)
        #[arg(short, long)]
        timeframe: Option<String>,

        /// Maximum concurrent instruments to download
        #[arg(long, default_value = "4")]
        parallel_instruments: usize,

        /// Maximum concurrent HTTP requests per instrument
        #[arg(long, default_value = "32")]
        concurrency: usize,

        /// Run in background as daemon
        #[arg(long)]
        background: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Manage background jobs (pause, resume, kill, clean)
    Job {
        #[command(subcommand)]
        action: JobAction,
    },
}

/// Actions for managing background jobs.
#[derive(Subcommand)]
enum JobAction {
    /// Pause a running job
    Pause {
        /// Job ID to pause
        job_id: String,
    },

    /// Resume a paused job
    Resume {
        /// Job ID to resume
        job_id: String,
    },

    /// Kill a running or paused job
    Kill {
        /// Job ID to kill
        job_id: String,
    },

    /// Clean up finished jobs from storage
    Clean {
        /// Clean all finished jobs (not just old ones)
        #[arg(long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check for daemon mode first (internal use)
    if let Some(job_id) = cli.daemon_run {
        return commands::daemon_run::daemon_run(&job_id).await;
    }

    // Show help if no command provided
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        return Ok(());
    };

    match command {
        Commands::Download {
            instrument,
            start,
            end,
            output,
            format,
            timeframe,
            concurrency,
            background,
            yes,
        } => {
            commands::download::download(
                &instrument,
                start.as_deref(),
                end.as_deref(),
                output,
                format,
                timeframe.as_deref(),
                concurrency,
                background,
                yes,
                cli.quiet,
            )
            .await
        }
        Commands::List { category, search } => {
            commands::list::list_instruments(category.as_deref(), search.as_deref())
        }
        Commands::Info { instrument } => commands::info::show_info(&instrument),
        Commands::Status {
            job_id,
            running,
            all,
            follow,
            cancel,
        } => commands::status::status(job_id.as_deref(), running, all, follow, cancel.as_deref()),
        Commands::DownloadAll {
            category,
            start,
            end,
            output_dir,
            format,
            timeframe,
            parallel_instruments,
            concurrency,
            background,
            yes,
        } => {
            commands::download_all::download_all(
                category.as_deref(),
                start.as_deref(),
                end.as_deref(),
                output_dir,
                format,
                timeframe.as_deref(),
                parallel_instruments,
                concurrency,
                background,
                yes,
                cli.quiet,
            )
            .await
        }
        Commands::Job { action } => match action {
            JobAction::Pause { job_id } => {
                commands::job::job_command("pause", Some(&job_id), false)
            }
            JobAction::Resume { job_id } => {
                commands::job::job_command("resume", Some(&job_id), false)
            }
            JobAction::Kill { job_id } => {
                commands::job::job_command("kill", Some(&job_id), false)
            }
            JobAction::Clean { all } => commands::job::job_command("clean", None, all),
        },
    }
}
