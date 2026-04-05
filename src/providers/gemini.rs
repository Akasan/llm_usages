use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use chrono::NaiveDate;
use serde::Deserialize;

use crate::provider::UsageProvider;
use crate::types::{TimeRange, UsageRecord};

pub struct GeminiProvider;

impl GeminiProvider {
    pub fn new() -> Self {
        Self
    }

    fn base_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".gemini").join("tmp"))
    }
}

#[derive(Debug, Deserialize)]
struct SessionFile {
    #[serde(default)]
    messages: Vec<SessionMessage>,
}

#[derive(Debug, Deserialize)]
struct SessionMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    tokens: Option<Tokens>,
    model: Option<String>,
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Tokens {
    #[serde(default)]
    input: u64,
    #[serde(default)]
    output: u64,
    #[serde(default)]
    cached: u64,
}

fn parse_date(ts: &str) -> Option<NaiveDate> {
    ts.get(..10)
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

/// Read the .project_root file from the session's parent directory
fn read_project_root(session_path: &std::path::Path) -> Option<String> {
    let parent = session_path.parent()?;
    let project_root_file = parent.join(".project_root");
    fs::read_to_string(project_root_file)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl UsageProvider for GeminiProvider {
    fn name(&self) -> &str {
        "Gemini"
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
            let fname = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if !fname.starts_with("session-") || !fname.ends_with(".json") {
                continue;
            }

            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let session: SessionFile = match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let project = read_project_root(path);

            for msg in &session.messages {
                if msg.msg_type.as_deref() != Some("gemini") {
                    continue;
                }

                let tokens = match &msg.tokens {
                    Some(t) => t,
                    None => continue,
                };

                let date = msg
                    .timestamp
                    .as_deref()
                    .and_then(parse_date)
                    .unwrap_or(range.from);

                if date < range.from || date > range.to {
                    continue;
                }

                let model = msg
                    .model
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

                records.push(UsageRecord {
                    provider: "Gemini".to_string(),
                    date,
                    model,
                    input_tokens: tokens.input,
                    output_tokens: tokens.output,
                    cache_creation_tokens: 0,
                    cache_read_tokens: tokens.cached,
                    project: project.clone(),
                });
            }
        }

        Ok(records)
    }
}
