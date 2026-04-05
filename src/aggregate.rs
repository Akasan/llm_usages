use std::collections::HashMap;

use chrono::NaiveDate;

use crate::pricing::estimate_cost;
use crate::types::{ProjectSummary, UsageRecord};

pub fn aggregate_by_date_model(records: Vec<UsageRecord>) -> Vec<UsageRecord> {
    let mut map: HashMap<(String, NaiveDate, String, Option<String>), UsageRecord> = HashMap::new();

    for r in records {
        let key = (r.provider.clone(), r.date, r.model.clone(), r.project.clone());
        let entry = map.entry(key).or_insert_with(|| UsageRecord {
            provider: r.provider.clone(),
            date: r.date,
            model: r.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            project: r.project.clone(),
        });
        entry.input_tokens += r.input_tokens;
        entry.output_tokens += r.output_tokens;
        entry.cache_creation_tokens += r.cache_creation_tokens;
        entry.cache_read_tokens += r.cache_read_tokens;
    }

    let mut result: Vec<UsageRecord> = map.into_values().collect();
    result.sort_by(|a, b| {
        a.provider
            .cmp(&b.provider)
            .then(a.date.cmp(&b.date))
            .then(a.model.cmp(&b.model))
    });
    result
}

fn display_name_from_path(path: &Option<String>) -> String {
    match path {
        Some(p) => {
            let trimmed = p.trim_end_matches('/');
            trimmed
                .rsplit('/')
                .next()
                .unwrap_or(trimmed)
                .to_string()
        }
        None => "(unknown)".to_string(),
    }
}

pub fn aggregate_by_project(records: &[UsageRecord]) -> Vec<ProjectSummary> {
    let mut map: HashMap<Option<String>, ProjectSummary> = HashMap::new();

    for r in records {
        let entry = map.entry(r.project.clone()).or_insert_with(|| ProjectSummary {
            display_name: display_name_from_path(&r.project),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_creation_tokens: 0,
            total_cache_read_tokens: 0,
            total_cost: 0.0,
            providers: Vec::new(),
        });
        entry.total_input_tokens += r.input_tokens;
        entry.total_output_tokens += r.output_tokens;
        entry.total_cache_creation_tokens += r.cache_creation_tokens;
        entry.total_cache_read_tokens += r.cache_read_tokens;
        entry.total_cost += estimate_cost(r);
        if !entry.providers.contains(&r.provider) {
            entry.providers.push(r.provider.clone());
        }
    }

    let mut result: Vec<ProjectSummary> = map.into_values().collect();
    // Sort by cost descending
    result.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap_or(std::cmp::Ordering::Equal));
    result
}
