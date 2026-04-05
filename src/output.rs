use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use tabled::{Table, Tabled, settings::Style};

use crate::pricing::estimate_cost;
use crate::types::{TimeRange, UsageRecord};

fn colorize_provider(provider: &str) -> String {
    match provider {
        "Claude" => format!("\x1b[35m{}\x1b[0m", provider), // Magenta
        "Codex" => format!("\x1b[32m{}\x1b[0m", provider),  // Green
        "Gemini" => format!("\x1b[36m{}\x1b[0m", provider),  // Cyan
        _ => provider.to_string(),
    }
}

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

#[derive(Tabled)]
struct DailySummaryRow {
    #[tabled(rename = "Date")]
    date: String,
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

pub(crate) fn truncate_model(model: &str, max_len: usize) -> String {
    if model.len() <= max_len {
        model.to_string()
    } else {
        format!("{}…", &model[..max_len - 1])
    }
}

pub(crate) fn format_tokens(n: u64) -> String {
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

pub fn print_table(records: &[UsageRecord], range: &TimeRange) {
    if records.is_empty() {
        println!("No usage data found.");
        return;
    }

    // Main table with colored provider names
    let rows: Vec<Row> = records
        .iter()
        .map(|r| {
            let cost = estimate_cost(r);
            Row {
                provider: colorize_provider(&r.provider),
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

    // Summary
    let total_input: u64 = records.iter().map(|r| r.input_tokens).sum();
    let total_output: u64 = records.iter().map(|r| r.output_tokens).sum();
    let total_cache_write: u64 = records.iter().map(|r| r.cache_creation_tokens).sum();
    let total_cache_read: u64 = records.iter().map(|r| r.cache_read_tokens).sum();
    let total_cost: f64 = records.iter().map(estimate_cost).sum();

    println!("\nSummary:");
    println!("  Total Input Tokens:  {}", format_tokens(total_input));
    println!("  Total Output Tokens: {}", format_tokens(total_output));
    println!("  Total Cache Write:   {}", format_tokens(total_cache_write));
    println!("  Total Cache Read:    {}", format_tokens(total_cache_read));
    println!("  Total Est. Cost:     ${:.4}", total_cost);

    // Daily summary
    print_daily_summary(records);

    // Monthly projection
    print_projection(records, range);
}

fn print_daily_summary(records: &[UsageRecord]) {
    let mut daily: BTreeMap<NaiveDate, (u64, u64, u64, u64, f64)> = BTreeMap::new();
    for r in records {
        let entry = daily.entry(r.date).or_insert((0, 0, 0, 0, 0.0));
        entry.0 += r.input_tokens;
        entry.1 += r.output_tokens;
        entry.2 += r.cache_creation_tokens;
        entry.3 += r.cache_read_tokens;
        entry.4 += estimate_cost(r);
    }

    let rows: Vec<DailySummaryRow> = daily
        .iter()
        .map(
            |(date, (input, output, cache_write, cache_read, cost))| DailySummaryRow {
                date: date.to_string(),
                input_tokens: format_tokens(*input),
                output_tokens: format_tokens(*output),
                cache_creation: format_tokens(*cache_write),
                cache_read: format_tokens(*cache_read),
                est_cost: format!("${:.4}", cost),
            },
        )
        .collect();

    println!("\nDaily Summary:");
    let table = Table::new(&rows).with(Style::rounded()).to_string();
    println!("{table}");
}

fn print_projection(records: &[UsageRecord], range: &TimeRange) {
    let today = chrono::Local::now().date_naive();
    let current_month = today.month();
    let current_year = today.year();

    // Filter records for current month
    let monthly_cost: f64 = records
        .iter()
        .filter(|r| r.date.month() == current_month && r.date.year() == current_year)
        .map(estimate_cost)
        .sum();

    if monthly_cost == 0.0 {
        return;
    }

    // Days elapsed in current month up to min(today, range.to)
    let month_start = NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap();
    let effective_end = std::cmp::min(today, range.to);
    let days_elapsed = (effective_end - month_start).num_days() + 1;

    if days_elapsed <= 0 {
        return;
    }

    // Days in current month
    let next_month = if current_month == 12 {
        NaiveDate::from_ymd_opt(current_year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(current_year, current_month + 1, 1).unwrap()
    };
    let days_in_month = (next_month - month_start).num_days();

    let daily_average = monthly_cost / days_elapsed as f64;
    let projected = daily_average * days_in_month as f64;

    let month_name = match current_month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };

    println!(
        "\nMonthly Projection ({} {}):",
        month_name, current_year
    );
    println!("  Days elapsed:        {}/{}", days_elapsed, days_in_month);
    println!("  Current total:       ${:.4}", monthly_cost);
    println!("  Daily average:       ${:.4}", daily_average);
    println!("  Projected monthly:   \x1b[1m${:.4}\x1b[0m", projected);
}
