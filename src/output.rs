use tabled::{Table, Tabled, settings::Style};

use crate::pricing::estimate_cost;
use crate::types::UsageRecord;

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "Provider")]
    provider: String,
    #[tabled(rename = "Date")]
    date: String,
    #[tabled(rename = "Model")]
    model: String,
    #[tabled(rename = "Input Tokens")]
    input_tokens: String,
    #[tabled(rename = "Output Tokens")]
    output_tokens: String,
    #[tabled(rename = "Cache Write")]
    cache_creation: String,
    #[tabled(rename = "Cache Read")]
    cache_read: String,
    #[tabled(rename = "Est. Cost (USD)")]
    est_cost: String,
}

fn truncate_model(model: &str, max_len: usize) -> String {
    if model.len() <= max_len {
        model.to_string()
    } else {
        format!("{}…", &model[..max_len - 1])
    }
}

fn format_tokens(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

pub fn print_table(records: &[UsageRecord]) {
    if records.is_empty() {
        println!("No usage data found.");
        return;
    }

    let rows: Vec<Row> = records
        .iter()
        .map(|r| {
            let cost = estimate_cost(r);
            Row {
                provider: r.provider.clone(),
                date: r.date.to_string(),
                model: truncate_model(&r.model, 24),
                input_tokens: format_tokens(r.input_tokens),
                output_tokens: format_tokens(r.output_tokens),
                cache_creation: format_tokens(r.cache_creation_tokens),
                cache_read: format_tokens(r.cache_read_tokens),
                est_cost: format!("${:.4}", cost),
            }
        })
        .collect();

    let table = Table::new(&rows).with(Style::rounded()).to_string();
    println!("{table}");

    let total_input: u64 = records.iter().map(|r| r.input_tokens).sum();
    let total_output: u64 = records.iter().map(|r| r.output_tokens).sum();
    let total_cache_write: u64 = records.iter().map(|r| r.cache_creation_tokens).sum();
    let total_cache_read: u64 = records.iter().map(|r| r.cache_read_tokens).sum();
    let total_cost: f64 = records.iter().map(estimate_cost).sum();

    println!("Summary:");
    println!("  Total Input Tokens:  {}", format_tokens(total_input));
    println!("  Total Output Tokens: {}", format_tokens(total_output));
    println!("  Total Cache Write:   {}", format_tokens(total_cache_write));
    println!("  Total Cache Read:    {}", format_tokens(total_cache_read));
    println!("  Total Est. Cost:     ${:.4}", total_cost);
}
