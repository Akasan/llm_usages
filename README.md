# llm_usages

A Rust CLI tool that displays LLM usage (token counts and estimated costs) from local session logs of Claude Code, Codex CLI, and Gemini CLI. No API keys required.

When run in a terminal, it launches an interactive TUI with tab-based navigation. When piped, it falls back to plain text output.

[Japanese / 日本語](README.ja.md)

## Data Sources

| CLI | Log Path | Format |
|-----|----------|--------|
| Claude Code | `~/.claude/projects/*/*.jsonl` | JSONL |
| Codex CLI | `~/.codex/sessions/YYYY/MM/DD/*.jsonl` | JSONL |
| Gemini CLI | `~/.gemini/tmp/*/chats/session-*.json` | JSON |

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Show usage for the past 7 days (default)
llm_usages

# Show usage for the past 30 days
llm_usages --days 30

# Specify a date range
llm_usages --from 2026-03-01 --to 2026-04-05

# Filter by provider
llm_usages -p claude
llm_usages -p claude,codex
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --days <N>` | Query the past N days | 7 |
| `--from <YYYY-MM-DD>` | Start date (overrides `--days`) | - |
| `--to <YYYY-MM-DD>` | End date | today |
| `-p, --provider <LIST>` | Target providers (comma-separated) | claude,codex,gemini |

## TUI Mode

When run in a terminal, an interactive TUI is displayed with three tabs:

| Tab | Content |
|-----|---------|
| **Detail** | Per-provider/date/model usage table with summary footer |
| **Daily Summary** | Aggregated daily totals + per-model bar chart (cost / tokens) with legend |
| **Projection** | Projected monthly cost based on current usage |

### Key Bindings

| Key | Action |
|-----|--------|
| `←/→` or `h/l` | Switch tabs |
| `1/2/3` | Jump to tab directly |
| `↑/↓` or `j/k` | Scroll |
| `PageUp/PageDown` | Scroll 10 lines |
| `t` | Toggle chart data (cost / tokens) |
| `q` or `Esc` | Quit |

### Plain Text Fallback

When piped (e.g. `llm_usages | cat`), the output falls back to plain text tables.

## Example Output (Plain Text)

```
╭──────────┬────────────┬────────────────────┬──────────────┬───────────────┬─────────────┬────────────┬─────────────────╮
│ Provider │ Date       │ Model              │ Input Tokens │ Output Tokens │ Cache Write │ Cache Read │ Est. Cost (USD) │
├──────────┼────────────┼────────────────────┼──────────────┼───────────────┼─────────────┼────────────┼─────────────────┤
│ Claude   │ 2026-04-04 │ claude-sonnet-4-20… │ 1,234,567   │ 456,789       │ 12,345      │ 890,123    │ $10.7890        │
│ Codex    │ 2026-04-04 │ gpt-5.3-codex      │ 141,201     │ 9,756         │ 0           │ 6,528      │ $0.4508         │
│ Gemini   │ 2026-04-04 │ gemini-3-flash-pre… │ 6,700       │ 35            │ 0           │ 0          │ $0.0010         │
╰──────────┴────────────┴────────────────────┴──────────────┴───────────────┴─────────────┴────────────┴─────────────────╯
Summary:
  Total Input Tokens:  1,382,468
  Total Output Tokens: 466,580
  Total Cache Write:   12,345
  Total Cache Read:    896,651
  Total Est. Cost:     $11.2408
```

## Supported Providers

| Provider | Log Source |
|----------|-----------|
| Claude Code | `~/.claude/projects/` |
| Codex CLI | `~/.codex/sessions/` |
| Gemini CLI | `~/.gemini/tmp/` |
