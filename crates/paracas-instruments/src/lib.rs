//! Instrument registry for paracas tick data downloader.
//!
//! This crate provides access to the full list of Dukascopy instruments
//! with their metadata including decimal factors for price normalization.
//!
//! # Example
//!
//! ```
//! use paracas_instruments::InstrumentRegistry;
//!
//! let registry = InstrumentRegistry::global();
//!
//! // Lookup by ID
//! if let Some(instrument) = registry.get("eurusd") {
//!     println!("{}: {}", instrument.name(), instrument.decimal_factor());
//! }
//! ```

#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/paracas/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::OnceLock;

use paracas_types::{Category, Instrument};

/// The instrument metadata JSON embedded at compile time.
const INSTRUMENTS_JSON: &str = include_str!("../data/instruments.json");

/// Global instrument registry instance.
static REGISTRY: OnceLock<InstrumentRegistry> = OnceLock::new();

/// Registry of all supported Dukascopy instruments.
#[derive(Debug)]
pub struct InstrumentRegistry {
    instruments: HashMap<String, Instrument>,
}

impl InstrumentRegistry {
    /// Returns the global instrument registry.
    ///
    /// The registry is initialized lazily on first access.
    #[must_use]
    pub fn global() -> &'static Self {
        REGISTRY.get_or_init(Self::load)
    }

    /// Loads instruments from the embedded JSON data.
    fn load() -> Self {
        let instruments: HashMap<String, Instrument> =
            serde_json::from_str(INSTRUMENTS_JSON).expect("Invalid instruments.json");
        Self { instruments }
    }

    /// Looks up an instrument by ID (case-insensitive).
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Instrument> {
        self.instruments.get(&id.to_lowercase())
    }

    /// Returns all instruments as an iterator.
    pub fn all(&self) -> impl Iterator<Item = &Instrument> {
        self.instruments.values()
    }

    /// Returns the total number of instruments.
    #[must_use]
    pub fn len(&self) -> usize {
        self.instruments.len()
    }

    /// Returns true if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }

    /// Returns all forex instruments.
    pub fn forex(&self) -> impl Iterator<Item = &Instrument> {
        self.instruments.values().filter(|i| i.is_forex())
    }

    /// Returns all cryptocurrency instruments.
    pub fn crypto(&self) -> impl Iterator<Item = &Instrument> {
        self.instruments.values().filter(|i| i.is_crypto())
    }

    /// Returns all index instruments.
    pub fn indices(&self) -> impl Iterator<Item = &Instrument> {
        self.instruments.values().filter(|i| i.is_index())
    }

    /// Returns all stock instruments.
    pub fn stocks(&self) -> impl Iterator<Item = &Instrument> {
        self.instruments.values().filter(|i| i.is_stock())
    }

    /// Returns all commodity instruments.
    pub fn commodities(&self) -> impl Iterator<Item = &Instrument> {
        self.instruments.values().filter(|i| i.is_commodity())
    }

    /// Returns instruments matching the given category.
    pub fn by_category(&self, category: Category) -> impl Iterator<Item = &Instrument> {
        self.instruments
            .values()
            .filter(move |i| i.category() == category)
    }

    /// Searches instruments by name or ID pattern (case-insensitive).
    pub fn search(&self, pattern: &str) -> Vec<&Instrument> {
        let pattern = pattern.to_lowercase();
        self.instruments
            .values()
            .filter(|i| {
                i.id().to_lowercase().contains(&pattern)
                    || i.name().to_lowercase().contains(&pattern)
            })
            .collect()
    }

    /// Returns all instrument IDs sorted alphabetically.
    pub fn ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.instruments.keys().map(String::as_str).collect();
        ids.sort();
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_loads() {
        let registry = InstrumentRegistry::global();
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_get_eurusd() {
        let registry = InstrumentRegistry::global();
        let eurusd = registry.get("eurusd").expect("EURUSD should exist");
        assert_eq!(eurusd.id(), "eurusd");
        assert_eq!(eurusd.decimal_factor(), 100_000);
    }

    #[test]
    fn test_get_case_insensitive() {
        let registry = InstrumentRegistry::global();
        assert!(registry.get("EURUSD").is_some());
        assert!(registry.get("EurUsd").is_some());
        assert!(registry.get("eurusd").is_some());
    }

    #[test]
    fn test_forex_filter() {
        let registry = InstrumentRegistry::global();
        let forex: Vec<_> = registry.forex().collect();
        assert!(!forex.is_empty());
        assert!(forex.iter().all(|i| i.is_forex()));
    }

    #[test]
    fn test_search() {
        let registry = InstrumentRegistry::global();
        let results = registry.search("eur");
        assert!(!results.is_empty());
    }
}
