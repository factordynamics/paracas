//! paracas CLI - High-performance Dukascopy tick data downloader.

use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueEnum};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use paracas_lib::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "paracas")]
#[command(about = "High-performance Dukascopy tick data downloader", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Quiet mode (suppress progress output)
    #[arg(short, long, global = true)]
    quiet: bool,
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
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Csv,
    Json,
    Ndjson,
    Parquet,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Download {
            instrument,
            start,
            end,
            output,
            format,
            timeframe,
            concurrency,
        } => {
            download(
                &instrument,
                start.as_deref(),
                end.as_deref(),
                output,
                format,
                timeframe.as_deref(),
                concurrency,
                cli.quiet,
            )
            .await
        }
        Commands::List { category, search } => {
            list_instruments(category.as_deref(), search.as_deref())
        }
        Commands::Info { instrument } => show_info(&instrument),
    }
}

#[allow(clippy::too_many_arguments)]
async fn download(
    instrument_id: &str,
    start_str: Option<&str>,
    end_str: Option<&str>,
    output: Option<PathBuf>,
    format: Format,
    timeframe_str: Option<&str>,
    concurrency: usize,
    quiet: bool,
) -> Result<()> {
    // Lookup instrument
    let registry = InstrumentRegistry::global();
    let instrument = registry
        .get(instrument_id)
        .with_context(|| format!("Unknown instrument: {instrument_id}"))?;

    // Parse start date (default to instrument's earliest available data)
    let start = match start_str {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .with_context(|| format!("Invalid start date: {s}"))?,
        None => instrument
            .start_tick_date()
            .map(|dt| dt.date_naive())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2003, 5, 5).expect("valid date")),
    };

    // Parse end date (default to today)
    let end = match end_str {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .with_context(|| format!("Invalid end date: {s}"))?,
        None => chrono::Utc::now().date_naive(),
    };

    let range = DateRange::new(start, end)?;

    // Determine output path (default to <instrument>.<format>)
    let output = output.unwrap_or_else(|| {
        let ext = match format {
            Format::Csv => "csv",
            Format::Json => "json",
            Format::Ndjson => "ndjson",
            Format::Parquet => "parquet",
        };
        PathBuf::from(format!("{}.{}", instrument_id, ext))
    });

    // Parse timeframe
    let timeframe = match timeframe_str {
        Some(tf) => tf
            .parse::<Timeframe>()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        None => Timeframe::Tick,
    };

    // Create client
    let config = ClientConfig {
        concurrency,
        ..Default::default()
    };
    let client = DownloadClient::new(config)?;

    // Setup progress bar
    let total_hours = range.total_hours() as u64;
    let progress = if quiet {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(total_hours);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} hours ({percent}%) {msg}")
                .expect("Invalid progress template")
                .progress_chars("=>-"),
        );
        pb.set_message(format!("{} {} -> {}", instrument.id(), start, end));
        pb
    };

    // Download and collect ticks using the resilient stream
    // This will retry on transient errors and skip hours that fail after retries
    let mut all_ticks: Vec<Tick> = Vec::new();
    let mut skipped_hours = 0u64;
    let mut stream = paracas_lib::tick_stream_resilient(&client, instrument, range);

    while let Some(batch) = stream.next().await {
        if batch.had_error() {
            skipped_hours += 1;
        }
        all_ticks.extend(batch.ticks);
        progress.inc(1);
    }

    let finish_msg = if skipped_hours > 0 {
        format!(
            "Downloaded {} ticks ({} hours skipped due to errors)",
            all_ticks.len(),
            skipped_hours
        )
    } else {
        format!("Downloaded {} ticks", all_ticks.len())
    };
    progress.finish_with_message(finish_msg);

    // Aggregate if needed
    if timeframe.is_tick() {
        // Write raw ticks
        write_ticks(&all_ticks, &output, format)?;
    } else {
        // Aggregate to OHLCV
        let bars = aggregate_ticks(&all_ticks, timeframe);
        write_ohlcv(&bars, &output, format)?;
    }

    if !quiet {
        println!("Output written to: {}", output.display());
    }

    Ok(())
}

fn aggregate_ticks(ticks: &[Tick], timeframe: Timeframe) -> Vec<Ohlcv> {
    let mut aggregator = TickAggregator::new(timeframe);
    let mut bars = Vec::new();

    for tick in ticks {
        if let Some(bar) = aggregator.process(*tick) {
            bars.push(bar);
        }
    }

    if let Some(bar) = aggregator.finish() {
        bars.push(bar);
    }

    bars
}

fn write_ticks(ticks: &[Tick], output: &PathBuf, format: Format) -> Result<()> {
    let file = File::create(output)?;
    let writer = BufWriter::new(file);

    match format {
        Format::Csv => {
            let formatter = CsvFormatter::new();
            formatter.write_ticks(ticks, writer)?;
        }
        Format::Json => {
            let formatter = JsonFormatter::new();
            formatter.write_ticks(ticks, writer)?;
        }
        Format::Ndjson => {
            let formatter = JsonFormatter::ndjson();
            formatter.write_ticks(ticks, writer)?;
        }
        Format::Parquet => {
            #[cfg(feature = "parquet")]
            {
                let formatter = ParquetFormatter::new();
                formatter.write_ticks(ticks, writer)?;
            }
            #[cfg(not(feature = "parquet"))]
            {
                bail!("Parquet support not compiled in");
            }
        }
    }

    Ok(())
}

fn write_ohlcv(bars: &[Ohlcv], output: &PathBuf, format: Format) -> Result<()> {
    let file = File::create(output)?;
    let writer = BufWriter::new(file);

    match format {
        Format::Csv => {
            let formatter = CsvFormatter::new();
            formatter.write_ohlcv(bars, writer)?;
        }
        Format::Json => {
            let formatter = JsonFormatter::new();
            formatter.write_ohlcv(bars, writer)?;
        }
        Format::Ndjson => {
            let formatter = JsonFormatter::ndjson();
            formatter.write_ohlcv(bars, writer)?;
        }
        Format::Parquet => {
            #[cfg(feature = "parquet")]
            {
                let formatter = ParquetFormatter::new();
                formatter.write_ohlcv(bars, writer)?;
            }
            #[cfg(not(feature = "parquet"))]
            {
                bail!("Parquet support not compiled in");
            }
        }
    }

    Ok(())
}

fn list_instruments(category: Option<&str>, search: Option<&str>) -> Result<()> {
    let registry = InstrumentRegistry::global();

    let instruments: Vec<_> = match (category, search) {
        (Some(cat), _) => {
            let category = parse_category(cat)?;
            registry.by_category(category).collect()
        }
        (_, Some(pattern)) => registry.search(pattern),
        (None, None) => registry.all().collect(),
    };

    if instruments.is_empty() {
        println!("No instruments found.");
        return Ok(());
    }

    println!("{:<15} {:<20} {:<10}", "ID", "NAME", "CATEGORY");
    println!("{}", "-".repeat(50));

    for instrument in &instruments {
        println!(
            "{:<15} {:<20} {:<10}",
            instrument.id(),
            instrument.name(),
            instrument.category()
        );
    }

    println!("\nTotal: {} instruments", instruments.len());
    Ok(())
}

fn show_info(instrument_id: &str) -> Result<()> {
    let registry = InstrumentRegistry::global();
    let instrument = registry
        .get(instrument_id)
        .with_context(|| format!("Unknown instrument: {instrument_id}"))?;

    println!("Instrument: {}", instrument.name());
    println!("ID:         {}", instrument.id());
    println!("Category:   {}", instrument.category());
    println!("Description: {}", instrument.description());
    println!("Decimal Factor: {}", instrument.decimal_factor());

    if let Some(start) = instrument.start_tick_date() {
        println!("Data Available From: {}", start.format("%Y-%m-%d"));
    }

    Ok(())
}

fn parse_category(s: &str) -> Result<Category> {
    match s.to_lowercase().as_str() {
        "forex" => Ok(Category::Forex),
        "crypto" => Ok(Category::Crypto),
        "index" => Ok(Category::Index),
        "stock" => Ok(Category::Stock),
        "commodity" => Ok(Category::Commodity),
        "etf" => Ok(Category::Etf),
        "bond" => Ok(Category::Bond),
        _ => bail!(
            "Unknown category: {}. Valid options: forex, crypto, index, stock, commodity, etf, bond",
            s
        ),
    }
}
