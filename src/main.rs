mod aggregate;
mod cli;
mod output;
mod pricing;
mod provider;
mod providers;
mod tui;
mod types;

use std::io::IsTerminal;

use clap::Parser;

use crate::aggregate::{aggregate_by_date_model, aggregate_by_project};
use crate::cli::Cli;
use crate::output::print_table;
use crate::provider::UsageProvider;
use crate::providers::claude::ClaudeProvider;
use crate::providers::codex::CodexProvider;
use crate::providers::gemini::GeminiProvider;
use crate::types::UsageRecord;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let range = cli.time_range();
    let requested = cli.providers();

    let mut providers: Vec<Box<dyn UsageProvider>> = Vec::new();

    for name in &requested {
        match name.as_str() {
            "claude" => providers.push(Box::new(ClaudeProvider::new())),
            "codex" => providers.push(Box::new(CodexProvider::new())),
            "gemini" => providers.push(Box::new(GeminiProvider::new())),
            other => eprintln!("Unknown provider: {}", other),
        }
    }

    if providers.is_empty() {
        eprintln!("No providers specified.");
        return Ok(());
    }

    let mut all_records: Vec<UsageRecord> = Vec::new();
    for p in &providers {
        match p.fetch_usage(&range) {
            Ok(records) => all_records.extend(records),
            Err(e) => eprintln!("Error fetching {} usage: {}", p.name(), e),
        }
    }

    // Apply project filter if specified
    if let Some(ref filter) = cli.project {
        let filter_lower = filter.to_lowercase();
        all_records.retain(|r| {
            r.project
                .as_ref()
                .map(|p| p.to_lowercase().contains(&filter_lower))
                .unwrap_or(false)
        });
    }

    let project_summaries = aggregate_by_project(&all_records);
    let all_records = aggregate_by_date_model(all_records);

    if std::io::stdout().is_terminal() {
        tui::run_tui(&all_records, &project_summaries, &range)?;
    } else {
        print_table(&all_records, &project_summaries, &range);
    }

    Ok(())
}
