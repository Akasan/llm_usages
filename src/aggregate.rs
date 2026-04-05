use std::collections::HashMap;

use chrono::NaiveDate;

use crate::types::UsageRecord;

pub fn aggregate_by_date_model(records: Vec<UsageRecord>) -> Vec<UsageRecord> {
    let mut map: HashMap<(String, NaiveDate, String), UsageRecord> = HashMap::new();

    for r in records {
        let key = (r.provider.clone(), r.date, r.model.clone());
        let entry = map.entry(key).or_insert_with(|| UsageRecord {
            provider: r.provider.clone(),
            date: r.date,
            model: r.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
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
