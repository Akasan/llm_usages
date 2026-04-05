use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub provider: String,
    pub date: NaiveDate,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}
