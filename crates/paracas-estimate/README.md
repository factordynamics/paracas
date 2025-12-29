# paracas-estimate

Download size and time estimation for the paracas tick data downloader.

## Features

- Historical size estimates per instrument category
- Download time estimation based on data volume
- Confidence levels for estimates

## Types

- `EstimateDatabase` - Database of historical size estimates per category
- `CategoryEstimate` - Size estimates for a single category
- `Estimator` - Computes download estimates for instruments and date ranges
- `DownloadEstimate` - Estimated download metrics
- `EstimateConfidence` - Confidence level of the estimate

## Usage

```rust,ignore
use paracas_estimate::{Estimator, EstimateDatabase};
use paracas_types::{Category, DateRange};

// Create an estimator with the default database
let db = EstimateDatabase::default();
let estimator = Estimator::new(&db);

// Estimate download for a date range
let range = DateRange::new(start, end)?;
let estimate = estimator.estimate(Category::Forex, &range);

println!("Estimated size: {} bytes", estimate.size_bytes);
println!("Confidence: {:?}", estimate.confidence);
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
