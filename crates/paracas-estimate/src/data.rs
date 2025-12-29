//! Estimate database with historical averages.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Embedded JSON data with historical size estimates.
const SIZE_ESTIMATES_JSON: &str = include_str!("../data/size_estimates.json");

/// Static estimate database instance.
static ESTIMATES: OnceLock<EstimateDatabase> = OnceLock::new();

/// Size estimate for a single instrument category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryEstimate {
    /// Category name (e.g., "forex", "crypto").
    pub category: String,
    /// Average compressed bytes per hour of data.
    pub avg_compressed_bytes_per_hour: u64,
    /// Average number of ticks per hour.
    pub avg_ticks_per_hour: u64,
    /// Multiplier for peak trading hours.
    pub peak_multiplier: f64,
}

impl CategoryEstimate {
    /// Creates a new category estimate.
    #[must_use]
    pub fn new(
        category: impl Into<String>,
        avg_compressed_bytes_per_hour: u64,
        avg_ticks_per_hour: u64,
        peak_multiplier: f64,
    ) -> Self {
        Self {
            category: category.into(),
            avg_compressed_bytes_per_hour,
            avg_ticks_per_hour,
            peak_multiplier,
        }
    }

    /// Returns the maximum compressed bytes per hour (at peak).
    #[must_use]
    pub fn max_compressed_bytes_per_hour(&self) -> u64 {
        (self.avg_compressed_bytes_per_hour as f64 * self.peak_multiplier) as u64
    }

    /// Returns the maximum ticks per hour (at peak).
    #[must_use]
    pub fn max_ticks_per_hour(&self) -> u64 {
        (self.avg_ticks_per_hour as f64 * self.peak_multiplier) as u64
    }
}

/// Raw JSON structure for deserialization.
#[derive(Debug, Deserialize)]
struct RawEstimateData {
    categories: HashMap<String, RawCategoryEstimate>,
}

/// Raw category estimate from JSON.
#[derive(Debug, Deserialize)]
struct RawCategoryEstimate {
    avg_compressed_bytes_per_hour: u64,
    avg_ticks_per_hour: u64,
    peak_multiplier: f64,
}

/// Database of historical size estimates per instrument category.
#[derive(Debug, Clone)]
pub struct EstimateDatabase {
    categories: HashMap<String, CategoryEstimate>,
}

impl EstimateDatabase {
    /// Returns the global estimate database instance.
    ///
    /// This lazily initializes the database from embedded JSON on first access.
    #[must_use]
    pub fn global() -> &'static Self {
        ESTIMATES.get_or_init(|| {
            Self::from_json(SIZE_ESTIMATES_JSON)
                .expect("embedded size_estimates.json should be valid")
        })
    }

    /// Creates an estimate database from JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is invalid.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let raw: RawEstimateData = serde_json::from_str(json)?;
        let categories = raw
            .categories
            .into_iter()
            .map(|(name, raw_est)| {
                let estimate = CategoryEstimate::new(
                    name.clone(),
                    raw_est.avg_compressed_bytes_per_hour,
                    raw_est.avg_ticks_per_hour,
                    raw_est.peak_multiplier,
                );
                (name, estimate)
            })
            .collect();
        Ok(Self { categories })
    }

    /// Returns the estimate for a category by name.
    #[must_use]
    pub fn get(&self, category: &str) -> Option<&CategoryEstimate> {
        self.categories.get(category)
    }

    /// Returns all available categories.
    pub fn categories(&self) -> impl Iterator<Item = &str> {
        self.categories.keys().map(String::as_str)
    }

    /// Returns the number of categories in the database.
    #[must_use]
    pub fn len(&self) -> usize {
        self.categories.len()
    }

    /// Returns true if the database is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.categories.is_empty()
    }

    /// Returns a default estimate for unknown categories.
    #[must_use]
    pub fn default_estimate() -> CategoryEstimate {
        CategoryEstimate::new("unknown", 50000, 3000, 2.0)
    }
}

impl Default for EstimateDatabase {
    fn default() -> Self {
        Self::global().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_database_loads() {
        let db = EstimateDatabase::global();
        assert!(!db.is_empty());
        assert_eq!(db.len(), 7);
    }

    #[test]
    fn test_category_estimates_present() {
        let db = EstimateDatabase::global();

        let forex = db.get("forex").expect("forex should exist");
        assert_eq!(forex.avg_compressed_bytes_per_hour, 75000);
        assert_eq!(forex.avg_ticks_per_hour, 5000);

        let crypto = db.get("crypto").expect("crypto should exist");
        assert_eq!(crypto.avg_compressed_bytes_per_hour, 150000);
        assert_eq!(crypto.avg_ticks_per_hour, 10000);
    }

    #[test]
    fn test_peak_calculations() {
        let estimate = CategoryEstimate::new("test", 100000, 5000, 2.0);

        assert_eq!(estimate.max_compressed_bytes_per_hour(), 200000);
        assert_eq!(estimate.max_ticks_per_hour(), 10000);
    }

    #[test]
    fn test_default_estimate() {
        let default = EstimateDatabase::default_estimate();
        assert_eq!(default.category, "unknown");
        assert_eq!(default.avg_compressed_bytes_per_hour, 50000);
    }
}
