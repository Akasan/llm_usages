use chrono::NaiveDate;
use clap::Parser;

use crate::types::TimeRange;

#[derive(Parser, Debug)]
#[command(name = "llm_usages", about = "Display LLM CLI usage and estimated costs from local session logs")]
pub struct Cli {
    /// Number of past days to query (default: 7)
    #[arg(short, long, default_value_t = 7)]
    pub days: u32,

    /// Start date (YYYY-MM-DD). Overrides --days.
    #[arg(long)]
    pub from: Option<NaiveDate>,

    /// End date (YYYY-MM-DD, default: today)
    #[arg(long)]
    pub to: Option<NaiveDate>,

    /// Target providers (comma-separated: claude,codex,gemini)
    #[arg(short, long)]
    pub provider: Option<String>,
}

impl Cli {
    pub fn time_range(&self) -> TimeRange {
        let today = chrono::Local::now().date_naive();
        let to = self.to.unwrap_or(today);
        let from = self
            .from
            .unwrap_or_else(|| to - chrono::Duration::days(i64::from(self.days)));
        TimeRange { from, to }
    }

    pub fn providers(&self) -> Vec<String> {
        match &self.provider {
            Some(p) => p.split(',').map(|s| s.trim().to_lowercase()).collect(),
            None => vec![
                "claude".to_string(),
                "codex".to_string(),
                "gemini".to_string(),
            ],
        }
    }
}
