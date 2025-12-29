//! List command implementation.
//!
//! This module handles listing available instruments with optional filtering.

use crate::display::parse_category;
use anyhow::Result;
use paracas_lib::prelude::*;

/// List available instruments with optional category filter or search pattern.
pub(crate) fn list_instruments(category: Option<&str>, search: Option<&str>) -> Result<()> {
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
