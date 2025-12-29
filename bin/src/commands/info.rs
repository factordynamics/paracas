//! Info command implementation.
//!
//! This module handles displaying detailed information about a specific instrument,
//! including size estimates for different time periods.

use anyhow::{Context, Result};
use paracas_estimate::Estimator;
use paracas_lib::prelude::*;

/// Show detailed information about an instrument, including size estimates.
pub(crate) fn show_info(instrument_id: &str) -> Result<()> {
    let registry = InstrumentRegistry::global();
    let instrument = registry
        .get(instrument_id)
        .with_context(|| format!("Unknown instrument: {instrument_id}"))?;

    // Basic info
    println!("Instrument: {}", instrument.name());
    println!("ID:         {}", instrument.id());
    println!("Category:   {}", instrument.category());
    println!("Description: {}", instrument.description());
    println!("Decimal Factor: {}", instrument.decimal_factor());

    if let Some(start) = instrument.start_tick_date() {
        println!("Data Available From: {}", start.format("%Y-%m-%d"));

        // Calculate estimates for different time periods
        let today = chrono::Utc::now().date_naive();
        let estimator = Estimator::global();

        println!("\nDownload Estimates:");
        println!(
            "{:<20} {:>12} {:>12} {:>12}",
            "PERIOD", "DOWNLOAD", "OUTPUT (CSV)", "EST. TIME"
        );
        println!("{}", "-".repeat(60));

        // Last 1 day
        if let Ok(range) = DateRange::new(today - chrono::Duration::days(1), today) {
            let est = estimator.estimate_single(instrument, &range);
            println!(
                "{:<20} {:>12} {:>12} {:>12}",
                "Last 1 day",
                Estimator::format_bytes(est.estimated_compressed_bytes),
                Estimator::format_bytes(est.estimated_output_bytes),
                Estimator::format_duration(est.estimated_duration),
            );
        }

        // Last 1 week
        if let Ok(range) = DateRange::new(today - chrono::Duration::days(7), today) {
            let est = estimator.estimate_single(instrument, &range);
            println!(
                "{:<20} {:>12} {:>12} {:>12}",
                "Last 1 week",
                Estimator::format_bytes(est.estimated_compressed_bytes),
                Estimator::format_bytes(est.estimated_output_bytes),
                Estimator::format_duration(est.estimated_duration),
            );
        }

        // Last 1 month
        if let Ok(range) = DateRange::new(today - chrono::Duration::days(30), today) {
            let est = estimator.estimate_single(instrument, &range);
            println!(
                "{:<20} {:>12} {:>12} {:>12}",
                "Last 1 month",
                Estimator::format_bytes(est.estimated_compressed_bytes),
                Estimator::format_bytes(est.estimated_output_bytes),
                Estimator::format_duration(est.estimated_duration),
            );
        }

        // Last 1 year
        if let Ok(range) = DateRange::new(today - chrono::Duration::days(365), today) {
            let est = estimator.estimate_single(instrument, &range);
            println!(
                "{:<20} {:>12} {:>12} {:>12}",
                "Last 1 year",
                Estimator::format_bytes(est.estimated_compressed_bytes),
                Estimator::format_bytes(est.estimated_output_bytes),
                Estimator::format_duration(est.estimated_duration),
            );
        }

        // Full history (from start to today)
        let start_date = start.date_naive();
        if let Ok(range) = DateRange::new(start_date, today) {
            let est = estimator.estimate_single(instrument, &range);
            let years = (today - start_date).num_days() as f64 / 365.25;
            println!(
                "{:<20} {:>12} {:>12} {:>12}",
                format!("Full history ({:.1}y)", years),
                Estimator::format_bytes(est.estimated_compressed_bytes),
                Estimator::format_bytes(est.estimated_output_bytes),
                Estimator::format_duration(est.estimated_duration),
            );
        }

        println!("\nNote: Estimates are based on historical averages and may vary.");
    }

    Ok(())
}
