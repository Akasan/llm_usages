# llm_usages

Claude Code、Codex CLI、Gemini CLIのローカルセッションログからLLM利用量（トークン数・推定コスト）を表示するRust CLIツール。APIキー不要。

ターミナルで実行するとタブ切り替え式のインタラクティブTUIが起動します。パイプ経由の場合はプレーンテキスト出力にフォールバックします。

## データソース

| CLI | ログパス | 形式 |
|-----|---------|------|
| Claude Code | `~/.claude/projects/*/*.jsonl` | JSONL |
| Codex CLI | `~/.codex/sessions/YYYY/MM/DD/*.jsonl` | JSONL |
| Gemini CLI | `~/.gemini/tmp/*/chats/session-*.json` | JSON |

## インストール

```bash
cargo build --release
```

## 使い方

```bash
# 過去7日分の利用量を表示（デフォルト）
llm_usages

# 過去30日分を表示
llm_usages --days 30

# 日付範囲を指定
llm_usages --from 2026-03-01 --to 2026-04-05

# 特定プロバイダのみ
llm_usages -p claude
llm_usages -p claude,codex
```

## オプション

| オプション | 説明 | デフォルト |
|-----------|------|-----------|
| `-d, --days <N>` | 過去N日分を取得 | 7 |
| `--from <YYYY-MM-DD>` | 開始日（`--days`より優先） | - |
| `--to <YYYY-MM-DD>` | 終了日 | 今日 |
| `-p, --provider <LIST>` | 対象プロバイダ（カンマ区切り） | claude,codex,gemini |

## TUIモード

ターミナルで実行すると、3つのタブを持つインタラクティブTUIが表示されます。

| タブ | 内容 |
|------|------|
| **Detail** | プロバイダ/日付/モデル別の詳細テーブル + サマリーフッター |
| **Daily Summary** | 日別の合計テーブル + モデル別バーチャート（コスト / トークン数）＋凡例 |
| **Projection** | 当月の使用量から月末コストを予測 |

### キーバインド

| キー | 動作 |
|------|------|
| `←/→` or `h/l` | タブ切替 |
| `1/2/3` | タブ直接ジャンプ |
| `↑/↓` or `j/k` | スクロール |
| `PageUp/PageDown` | 10行スクロール |
| `t` | チャートのデータ切替（コスト / トークン数） |
| `q` or `Esc` | 終了 |

### プレーンテキストフォールバック

パイプ経由（例: `llm_usages | cat`）の場合は従来のテキストテーブルで出力されます。

## 出力例（プレーンテキスト）

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

## 対応プロバイダ

| プロバイダ | ログ読み取り元 |
|-----------|--------------|
| Claude Code | `~/.claude/projects/` |
| Codex CLI | `~/.codex/sessions/` |
| Gemini CLI | `~/.gemini/tmp/` |
