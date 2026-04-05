use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use chrono::NaiveDate;
use serde::Deserialize;

use crate::provider::UsageProvider;
use crate::types::{TimeRange, UsageRecord};

pub struct CodexProvider;

impl CodexProvider {
    pub fn new() -> Self {
        Self
    }

    fn base_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".codex").join("sessions"))
    }
}

#[derive(Debug, Deserialize)]
struct LogLine {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    line_type: Option<String>,
    payload: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    total_token_usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
struct TokenUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

fn parse_date(ts: &str) -> Option<NaiveDate> {
    ts.get(..10)
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

impl UsageProvider for CodexProvider {
    fn name(&self) -> &str {
        "Codex"
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

            let mut model = String::from("unknown");
            let mut last_token_usage: Option<TokenUsage> = None;
            let mut session_date: Option<NaiveDate> = None;
            let mut project: Option<String> = None;

            for line in content.lines() {
                let entry: LogLine = match serde_json::from_str(line) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                if session_date.is_none() {
                    if let Some(ts) = &entry.timestamp {
                        session_date = parse_date(ts);
                    }
                }

                let payload = match &entry.payload {
                    Some(p) => p,
                    None => continue,
                };

                // Extract project from session_meta cwd
                if entry.line_type.as_deref() == Some("session_meta") {
                    if let Some(cwd) = payload.get("cwd").and_then(|v| v.as_str()) {
                        if !cwd.is_empty() {
                            project = Some(cwd.to_string());
                        }
                    }
                }

                // Extract model from turn_context
                if entry.line_type.as_deref() == Some("turn_context") {
                    if let Some(m) = payload.get("model").and_then(|v| v.as_str()) {
                        model = m.to_string();
                    }
                }

                // Extract cumulative token count from the last token_count event
                if entry.line_type.as_deref() == Some("event_msg") {
                    if payload.get("type").and_then(|v| v.as_str()) == Some("token_count") {
                        if let Some(info) = payload.get("info") {
                            if !info.is_null() {
                                if let Ok(ti) = serde_json::from_value::<TokenInfo>(info.clone()) {
                                    if let Some(usage) = ti.total_token_usage {
                                        last_token_usage = Some(usage);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let date = session_date.unwrap_or(range.from);
            if date < range.from || date > range.to {
                continue;
            }

            if let Some(usage) = last_token_usage {
                records.push(UsageRecord {
                    provider: "Codex".to_string(),
                    date,
                    model,
                    input_tokens: usage.input_tokens,
                    output_tokens: usage.output_tokens,
                    cache_creation_tokens: 0,
                    cache_read_tokens: usage.cached_input_tokens,
                    project,
                });
            }
        }

        Ok(records)
    }
}
