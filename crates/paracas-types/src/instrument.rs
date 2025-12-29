//! Financial instrument definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Instrument category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// Foreign exchange currency pairs.
    Forex,
    /// Cryptocurrencies.
    Crypto,
    /// Stock indices.
    Index,
    /// Individual stocks.
    Stock,
    /// Commodities (metals, energy, agriculture).
    Commodity,
    /// Exchange-traded funds.
    Etf,
    /// Government bonds.
    Bond,
}

impl Category {
    /// Returns the category as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Forex => "forex",
            Self::Crypto => "crypto",
            Self::Index => "index",
            Self::Stock => "stock",
            Self::Commodity => "commodity",
            Self::Etf => "etf",
            Self::Bond => "bond",
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a tradable financial instrument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    /// Unique identifier (e.g., "eurusd", "btcusd").
    id: String,
    /// Human-readable name (e.g., "EUR/USD").
    name: String,
    /// Description of the instrument.
    description: String,
    /// Instrument category.
    category: Category,
    /// Decimal factor for price normalization.
    decimal_factor: u32,
    /// Earliest available tick data timestamp.
    start_tick_date: Option<DateTime<Utc>>,
}

impl Instrument {
    /// Creates a new instrument.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: Category,
        decimal_factor: u32,
        start_tick_date: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category,
            decimal_factor,
            start_tick_date,
        }
    }

    /// Returns the instrument identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the human-readable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the instrument category.
    #[must_use]
    pub const fn category(&self) -> Category {
        self.category
    }

    /// Returns the decimal factor for price normalization.
    #[must_use]
    pub const fn decimal_factor(&self) -> u32 {
        self.decimal_factor
    }

    /// Returns the decimal factor as f64 for price calculations.
    #[must_use]
    pub fn decimal_factor_f64(&self) -> f64 {
        f64::from(self.decimal_factor)
    }

    /// Returns the earliest available tick data timestamp.
    #[must_use]
    pub const fn start_tick_date(&self) -> Option<DateTime<Utc>> {
        self.start_tick_date
    }

    /// Returns true if tick data is available for the given date.
    #[must_use]
    pub fn has_data_for(&self, date: DateTime<Utc>) -> bool {
        self.start_tick_date.is_some_and(|start| date >= start)
    }

    /// Returns true if this is a forex instrument.
    #[must_use]
    pub const fn is_forex(&self) -> bool {
        matches!(self.category, Category::Forex)
    }

    /// Returns true if this is a cryptocurrency instrument.
    #[must_use]
    pub const fn is_crypto(&self) -> bool {
        matches!(self.category, Category::Crypto)
    }

    /// Returns true if this is an index instrument.
    #[must_use]
    pub const fn is_index(&self) -> bool {
        matches!(self.category, Category::Index)
    }

    /// Returns true if this is a stock instrument.
    #[must_use]
    pub const fn is_stock(&self) -> bool {
        matches!(self.category, Category::Stock)
    }

    /// Returns true if this is a commodity instrument.
    #[must_use]
    pub const fn is_commodity(&self) -> bool {
        matches!(self.category, Category::Commodity)
    }
}

impl std::fmt::Display for Instrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_instrument_creation() {
        let start = Utc.with_ymd_and_hms(2003, 5, 5, 0, 0, 0).unwrap();
        let instrument = Instrument::new(
            "eurusd",
            "EUR/USD",
            "Euro vs US Dollar",
            Category::Forex,
            100_000,
            Some(start),
        );

        assert_eq!(instrument.id(), "eurusd");
        assert_eq!(instrument.name(), "EUR/USD");
        assert_eq!(instrument.decimal_factor(), 100_000);
        assert!(instrument.is_forex());
        assert!(!instrument.is_crypto());
    }

    #[test]
    fn test_has_data_for() {
        let start = Utc.with_ymd_and_hms(2003, 5, 5, 0, 0, 0).unwrap();
        let instrument = Instrument::new(
            "eurusd",
            "EUR/USD",
            "Euro vs US Dollar",
            Category::Forex,
            100_000,
            Some(start),
        );

        let before = Utc.with_ymd_and_hms(2003, 1, 1, 0, 0, 0).unwrap();
        let after = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        assert!(!instrument.has_data_for(before));
        assert!(instrument.has_data_for(after));
    }
}
