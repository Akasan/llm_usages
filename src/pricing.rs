use crate::types::UsageRecord;

struct ModelPrice {
    prefix: &'static str,
    input_per_mtok: f64,
    output_per_mtok: f64,
}

const PRICES: &[ModelPrice] = &[
    // Anthropic (Claude Code)
    ModelPrice { prefix: "claude-opus-4", input_per_mtok: 15.0, output_per_mtok: 75.0 },
    ModelPrice { prefix: "claude-sonnet-4", input_per_mtok: 3.0, output_per_mtok: 15.0 },
    ModelPrice { prefix: "claude-haiku-4", input_per_mtok: 0.80, output_per_mtok: 4.0 },
    ModelPrice { prefix: "claude-3-5-sonnet", input_per_mtok: 3.0, output_per_mtok: 15.0 },
    ModelPrice { prefix: "claude-3-5-haiku", input_per_mtok: 0.80, output_per_mtok: 4.0 },
    ModelPrice { prefix: "claude-3-opus", input_per_mtok: 15.0, output_per_mtok: 75.0 },
    ModelPrice { prefix: "claude-3-sonnet", input_per_mtok: 3.0, output_per_mtok: 15.0 },
    ModelPrice { prefix: "claude-3-haiku", input_per_mtok: 0.25, output_per_mtok: 1.25 },
    // OpenAI (Codex CLI)
    ModelPrice { prefix: "gpt-5", input_per_mtok: 2.50, output_per_mtok: 10.0 },
    ModelPrice { prefix: "gpt-4o-mini", input_per_mtok: 0.15, output_per_mtok: 0.60 },
    ModelPrice { prefix: "gpt-4o", input_per_mtok: 2.50, output_per_mtok: 10.0 },
    ModelPrice { prefix: "gpt-4-turbo", input_per_mtok: 10.0, output_per_mtok: 30.0 },
    ModelPrice { prefix: "gpt-4", input_per_mtok: 30.0, output_per_mtok: 60.0 },
    ModelPrice { prefix: "gpt-3.5-turbo", input_per_mtok: 0.50, output_per_mtok: 1.50 },
    ModelPrice { prefix: "o3-mini", input_per_mtok: 1.10, output_per_mtok: 4.40 },
    ModelPrice { prefix: "o3", input_per_mtok: 10.0, output_per_mtok: 40.0 },
    ModelPrice { prefix: "o1-mini", input_per_mtok: 3.0, output_per_mtok: 12.0 },
    ModelPrice { prefix: "o1", input_per_mtok: 15.0, output_per_mtok: 60.0 },
    ModelPrice { prefix: "codex-mini", input_per_mtok: 1.50, output_per_mtok: 6.0 },
    // Google (Gemini CLI)
    ModelPrice { prefix: "gemini-2.5-pro", input_per_mtok: 1.25, output_per_mtok: 10.0 },
    ModelPrice { prefix: "gemini-2.5-flash", input_per_mtok: 0.15, output_per_mtok: 0.60 },
    ModelPrice { prefix: "gemini-2.0-flash", input_per_mtok: 0.10, output_per_mtok: 0.40 },
    ModelPrice { prefix: "gemini-3-flash", input_per_mtok: 0.15, output_per_mtok: 0.60 },
    ModelPrice { prefix: "gemini-3-pro", input_per_mtok: 1.25, output_per_mtok: 10.0 },
    ModelPrice { prefix: "gemini-1.5-pro", input_per_mtok: 1.25, output_per_mtok: 5.0 },
    ModelPrice { prefix: "gemini-1.5-flash", input_per_mtok: 0.075, output_per_mtok: 0.30 },
];

const DEFAULT_INPUT_PER_MTOK: f64 = 3.0;
const DEFAULT_OUTPUT_PER_MTOK: f64 = 15.0;

/// Cache read tokens are charged at 10% of normal input rate.
const CACHE_READ_DISCOUNT: f64 = 0.1;
/// Cache creation tokens are charged at 125% of normal input rate.
const CACHE_CREATION_MULTIPLIER: f64 = 1.25;

fn find_price(model: &str) -> (f64, f64) {
    let model_lower = model.to_lowercase();
    for p in PRICES {
        if model_lower.starts_with(p.prefix) {
            return (p.input_per_mtok, p.output_per_mtok);
        }
    }
    (DEFAULT_INPUT_PER_MTOK, DEFAULT_OUTPUT_PER_MTOK)
}

pub fn estimate_cost(record: &UsageRecord) -> f64 {
    let (input_rate, output_rate) = find_price(&record.model);
    let input_cost = (record.input_tokens as f64 / 1_000_000.0) * input_rate;
    let output_cost = (record.output_tokens as f64 / 1_000_000.0) * output_rate;
    let cache_read_cost =
        (record.cache_read_tokens as f64 / 1_000_000.0) * input_rate * CACHE_READ_DISCOUNT;
    let cache_creation_cost =
        (record.cache_creation_tokens as f64 / 1_000_000.0) * input_rate * CACHE_CREATION_MULTIPLIER;
    input_cost + output_cost + cache_read_cost + cache_creation_cost
}
