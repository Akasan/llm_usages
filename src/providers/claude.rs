use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use chrono::NaiveDate;
use serde::Deserialize;

use crate::provider::UsageProvider;
use crate::types::{TimeRange, UsageRecord};

pub struct ClaudeProvider;

impl ClaudeProvider {
    pub fn new() -> Self {
        Self
    }

    fn base_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".claude").join("projects"))
    }
}

#[derive(Debug, Deserialize)]
struct LogLine {
    #[serde(rename = "type")]
    line_type: Option<String>,
    message: Option<Message>,
    timestamp: Option<String>,
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

fn parse_date(ts: &str) -> Option<NaiveDate> {
    // ISO 8601: "2026-03-08T08:46:36.102Z"
    ts.get(..10)
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

/// Convert a Claude project directory name like `-Users-foo-bar` to `/Users/foo/bar`
fn dir_name_to_path(dir_name: &str) -> Option<String> {
    if dir_name.starts_with('-') {
        Some(dir_name.replace('-', "/"))
    } else {
        None
    }
}

impl UsageProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "Claude"
    }

    fn fetch_usage(&self, range: &TimeRange) -> Result<Vec<UsageRecord>> {
        let base = match Self::base_dir() {
            Some(p) if p.exists() => p,
            _ => return Ok(vec![]),
        };

        let mut records = Vec::new();

        for entry in walkdir::WalkDir::new(&base)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Extract project: prefer cwd from log lines, fallback to directory name
            let mut project: Option<String> = None;

            // Deduplicate by message.id: keep last occurrence (final streaming state)
            let mut msg_map: HashMap<String, (NaiveDate, String, Usage)> = HashMap::new();

            for line in content.lines() {
                let entry: LogLine = match serde_json::from_str(line) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                // Extract cwd from the first line that has it
                if project.is_none() {
                    if let Some(cwd) = &entry.cwd {
                        if !cwd.is_empty() {
                            project = Some(cwd.clone());
                        }
                    }
                }

                if entry.line_type.as_deref() != Some("assistant") {
                    continue;
                }

                let msg = match entry.message {
                    Some(m) => m,
                    None => continue,
                };

                let usage = match msg.usage {
                    Some(u) => u,
                    None => continue,
                };

                let date = entry
                    .timestamp
                    .as_deref()
                    .and_then(parse_date)
                    .unwrap_or(range.from);

                if date < range.from || date > range.to {
                    continue;
                }

                let model = msg.model.unwrap_or_else(|| "unknown".to_string());
                if model.starts_with('<') {
                    continue;
                }

                if let Some(id) = msg.id {
                    msg_map.insert(id, (date, model, usage));
                } else {
                    records.push(UsageRecord {
                        provider: "Claude".to_string(),
                        date,
                        model,
                        input_tokens: usage.input_tokens,
                        output_tokens: usage.output_tokens,
                        cache_creation_tokens: usage.cache_creation_input_tokens,
                        cache_read_tokens: usage.cache_read_input_tokens,
                        project: project.clone(),
                    });
                }
            }

            // Fallback: derive project from directory name if cwd was not found
            if project.is_none() {
                if let Ok(relative) = path.strip_prefix(&base) {
                    if let Some(first_component) = relative.components().next() {
                        let dir_name = first_component.as_os_str().to_string_lossy();
                        project = dir_name_to_path(&dir_name);
                    }
                }
            }

            for (_id, (date, model, usage)) in msg_map {
                records.push(UsageRecord {
                    provider: "Claude".to_string(),
                    date,
                    model,
                    input_tokens: usage.input_tokens,
                    output_tokens: usage.output_tokens,
                    cache_creation_tokens: usage.cache_creation_input_tokens,
                    cache_read_tokens: usage.cache_read_input_tokens,
                    project: project.clone(),
                });
            }
        }

        Ok(records)
    }
}
