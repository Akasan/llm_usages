use anyhow::Result;

use crate::types::{TimeRange, UsageRecord};

pub trait UsageProvider {
    fn name(&self) -> &str;
    fn fetch_usage(&self, range: &TimeRange) -> Result<Vec<UsageRecord>>;
}
