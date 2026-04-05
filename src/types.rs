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
    pub project: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub display_name: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cost: f64,
    pub providers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}
