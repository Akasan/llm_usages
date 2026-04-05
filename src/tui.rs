use std::collections::{BTreeMap, HashMap};
use std::io;

use chrono::{Datelike, NaiveDate};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Borders, Cell, Paragraph, Row, Table, Tabs};
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::Terminal;

use crate::output::{format_tokens, truncate_model};
use crate::pricing::estimate_cost;
use crate::types::{ProjectSummary, TimeRange, UsageRecord};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TabId {
    Detail,
    DailySummary,
    Projects,
    Projection,
    ModelShare,
}

impl TabId {
    const ALL: [TabId; 5] = [
        TabId::Detail,
        TabId::DailySummary,
        TabId::Projects,
        TabId::Projection,
        TabId::ModelShare,
    ];

    fn index(self) -> usize {
        match self {
            TabId::Detail => 0,
            TabId::DailySummary => 1,
            TabId::Projects => 2,
            TabId::Projection => 3,
            TabId::ModelShare => 4,
        }
    }

    fn from_index(i: usize) -> Self {
        TabId::ALL[i % TabId::ALL.len()]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChartMode {
    Cost,
    Tokens,
}

impl ChartMode {
    fn toggle(self) -> Self {
        match self {
            ChartMode::Cost => ChartMode::Tokens,
            ChartMode::Tokens => ChartMode::Cost,
        }
    }

    fn label(self) -> &'static str {
        match self {
            ChartMode::Cost => "Est. Cost (USD)",
            ChartMode::Tokens => "Total Tokens (Input + Output)",
        }
    }
}

fn provider_color(provider: &str) -> Color {
    match provider {
        "Claude" => Color::Magenta,
        "Codex" => Color::Green,
        "Gemini" => Color::Cyan,
        _ => Color::White,
    }
}

fn model_color(model: &str) -> Color {
    let m = model.to_lowercase();
    // High-end
    if m.contains("opus")
        || m.contains("-pro")
        || m.starts_with("gpt-5")
        || m.starts_with("gpt-4-turbo")
        || (m.starts_with("gpt-4") && !m.starts_with("gpt-4o"))
        || (m.starts_with("o3") && !m.starts_with("o3-mini"))
        || (m.starts_with("o1") && !m.starts_with("o1-mini"))
    {
        Color::Yellow
    // Mid-tier
    } else if m.contains("sonnet")
        || (m.starts_with("gpt-4o") && !m.contains("mini"))
    {
        Color::Blue
    // Lightweight
    } else {
        Color::DarkGray
    }
}

const MODEL_PALETTE: &[Color] = &[
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
];

struct ChartEntry {
    model_index: usize,
    cost: f64,
    total_tokens: u64,
}

struct ChartGroup {
    date_short: String,
    entries: Vec<ChartEntry>,
}

struct PieSlice {
    model: String,
    value: f64,
    percentage: f64,
    color: Color,
}

struct DetailRow {
    provider: String,
    date: String,
    model: String,
    model_raw: String,
    input_tokens: String,
    output_tokens: String,
    cache_write: String,
    cache_read: String,
    est_cost: String,
}

struct DailySummaryRow {
    date: String,
    input_tokens: String,
    output_tokens: String,
    cache_write: String,
    cache_read: String,
    est_cost: String,
}

struct ProjectRow {
    project: String,
    providers: Vec<String>,
    input_tokens: String,
    output_tokens: String,
    cache_write: String,
    cache_read: String,
    est_cost: String,
}

struct App {
    active_tab: TabId,
    chart_mode: ChartMode,
    detail_rows: Vec<DetailRow>,
    summary_footer: Vec<String>,
    daily_rows: Vec<DailySummaryRow>,
    chart_groups: Vec<ChartGroup>,
    model_legend: Vec<(String, Color)>,
    projection_lines: Vec<String>,
    project_rows: Vec<ProjectRow>,
    pie_cost: Vec<PieSlice>,
    pie_tokens: Vec<PieSlice>,
    detail_scroll: u16,
    daily_scroll: u16,
    projection_scroll: u16,
    project_scroll: u16,
    model_share_scroll: u16,
}

impl App {
    fn new(records: &[UsageRecord], project_summaries: &[ProjectSummary], range: &TimeRange) -> Self {
        let detail_rows = build_detail_rows(records);
        let summary_footer = build_summary_footer(records);
        let daily_rows = build_daily_rows(records);
        let (chart_groups, model_legend) = build_chart_data(records);
        let projection_lines = build_projection_lines(records, range);
        let project_rows = build_project_rows(project_summaries);
        let (pie_cost, pie_tokens) = build_pie_data(records);

        App {
            active_tab: TabId::Detail,
            chart_mode: ChartMode::Cost,
            detail_rows,
            summary_footer,
            daily_rows,
            chart_groups,
            model_legend,
            projection_lines,
            project_rows,
            pie_cost,
            pie_tokens,
            detail_scroll: 0,
            daily_scroll: 0,
            projection_scroll: 0,
            project_scroll: 0,
            model_share_scroll: 0,
        }
    }

    fn scroll_mut(&mut self) -> &mut u16 {
        match self.active_tab {
            TabId::Detail => &mut self.detail_scroll,
            TabId::DailySummary => &mut self.daily_scroll,
            TabId::Projects => &mut self.project_scroll,
            TabId::Projection => &mut self.projection_scroll,
            TabId::ModelShare => &mut self.model_share_scroll,
        }
    }

    fn content_len(&self) -> usize {
        match self.active_tab {
            TabId::Detail => self.detail_rows.len() + self.summary_footer.len() + 1,
            TabId::DailySummary => self.daily_rows.len(),
            TabId::Projects => self.project_rows.len(),
            TabId::Projection => self.projection_lines.len(),
            TabId::ModelShare => 0,
        }
    }

    fn scroll_down(&mut self, n: u16) {
        let scroll = self.scroll_mut();
        *scroll = scroll.saturating_add(n);
    }

    fn scroll_up(&mut self, n: u16) {
        let scroll = self.scroll_mut();
        *scroll = scroll.saturating_sub(n);
    }

    fn next_tab(&mut self) {
        let idx = self.active_tab.index();
        self.active_tab = TabId::from_index((idx + 1) % TabId::ALL.len());
    }

    fn prev_tab(&mut self) {
        let idx = self.active_tab.index();
        self.active_tab = TabId::from_index((idx + TabId::ALL.len() - 1) % TabId::ALL.len());
    }
}

fn build_detail_rows(records: &[UsageRecord]) -> Vec<DetailRow> {
    records
        .iter()
        .map(|r| {
            let cost = estimate_cost(r);
            DetailRow {
                provider: r.provider.clone(),
                date: r.date.to_string(),
                model: truncate_model(&r.model, 24),
                model_raw: r.model.clone(),
                input_tokens: format_tokens(r.input_tokens),
                output_tokens: format_tokens(r.output_tokens),
                cache_write: format_tokens(r.cache_creation_tokens),
                cache_read: format_tokens(r.cache_read_tokens),
                est_cost: format!("${:.4}", cost),
            }
        })
        .collect()
}

fn build_summary_footer(records: &[UsageRecord]) -> Vec<String> {
    if records.is_empty() {
        return vec![];
    }
    let total_input: u64 = records.iter().map(|r| r.input_tokens).sum();
    let total_output: u64 = records.iter().map(|r| r.output_tokens).sum();
    let total_cache_write: u64 = records.iter().map(|r| r.cache_creation_tokens).sum();
    let total_cache_read: u64 = records.iter().map(|r| r.cache_read_tokens).sum();
    let total_cost: f64 = records.iter().map(estimate_cost).sum();

    vec![
        format!("Total Input Tokens:  {}", format_tokens(total_input)),
        format!("Total Output Tokens: {}", format_tokens(total_output)),
        format!("Total Cache Write:   {}", format_tokens(total_cache_write)),
        format!("Total Cache Read:    {}", format_tokens(total_cache_read)),
        format!("Total Est. Cost:     ${:.4}", total_cost),
    ]
}

fn build_daily_rows(records: &[UsageRecord]) -> Vec<DailySummaryRow> {
    let mut daily: BTreeMap<NaiveDate, (u64, u64, u64, u64, f64)> = BTreeMap::new();
    for r in records {
        let entry = daily.entry(r.date).or_insert((0, 0, 0, 0, 0.0));
        entry.0 += r.input_tokens;
        entry.1 += r.output_tokens;
        entry.2 += r.cache_creation_tokens;
        entry.3 += r.cache_read_tokens;
        entry.4 += estimate_cost(r);
    }

    daily
        .iter()
        .map(|(date, (input, output, cw, cr, cost))| DailySummaryRow {
            date: date.to_string(),
            input_tokens: format_tokens(*input),
            output_tokens: format_tokens(*output),
            cache_write: format_tokens(*cw),
            cache_read: format_tokens(*cr),
            est_cost: format!("${:.4}", cost),
        })
        .collect()
}

fn build_chart_data(records: &[UsageRecord]) -> (Vec<ChartGroup>, Vec<(String, Color)>) {
    let mut model_order: Vec<String> = Vec::new();
    let mut model_map: HashMap<String, usize> = HashMap::new();
    let mut daily: BTreeMap<NaiveDate, BTreeMap<usize, (f64, u64)>> = BTreeMap::new();

    for r in records {
        let idx = if let Some(&i) = model_map.get(&r.model) {
            i
        } else {
            let i = model_order.len();
            model_map.insert(r.model.clone(), i);
            model_order.push(r.model.clone());
            i
        };

        let day = daily.entry(r.date).or_default();
        let entry = day.entry(idx).or_insert((0.0, 0));
        entry.0 += estimate_cost(r);
        entry.1 += r.input_tokens + r.output_tokens;
    }

    let groups = daily
        .iter()
        .map(|(date, models)| {
            let date_short = format!("{:02}/{:02}", date.month(), date.day());
            let mut entries: Vec<ChartEntry> = models
                .iter()
                .map(|(&idx, &(cost, tokens))| ChartEntry {
                    model_index: idx,
                    cost,
                    total_tokens: tokens,
                })
                .collect();
            entries.sort_by_key(|e| e.model_index);
            ChartGroup { date_short, entries }
        })
        .collect();

    let legend = model_order
        .iter()
        .enumerate()
        .map(|(i, model)| {
            let color = MODEL_PALETTE[i % MODEL_PALETTE.len()];
            (truncate_model(model, 24), color)
        })
        .collect();

    (groups, legend)
}

fn build_projection_lines(records: &[UsageRecord], range: &TimeRange) -> Vec<String> {
    let today = chrono::Local::now().date_naive();
    let current_month = today.month();
    let current_year = today.year();

    let monthly_cost: f64 = records
        .iter()
        .filter(|r| r.date.month() == current_month && r.date.year() == current_year)
        .map(estimate_cost)
        .sum();

    if monthly_cost == 0.0 {
        return vec!["No data for the current month.".to_string()];
    }

    let month_start = NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap();
    let effective_end = std::cmp::min(today, range.to);
    let days_elapsed = (effective_end - month_start).num_days() + 1;

    if days_elapsed <= 0 {
        return vec!["No data for the current month.".to_string()];
    }

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

    vec![
        format!("Monthly Projection ({} {})", month_name, current_year),
        String::new(),
        format!("  Days elapsed:        {}/{}", days_elapsed, days_in_month),
        format!("  Current total:       ${:.4}", monthly_cost),
        format!("  Daily average:       ${:.4}", daily_average),
        format!("  Projected monthly:   ${:.4}", projected),
    ]
}

fn build_project_rows(summaries: &[ProjectSummary]) -> Vec<ProjectRow> {
    summaries
        .iter()
        .map(|s| ProjectRow {
            project: s.display_name.clone(),
            providers: s.providers.clone(),
            input_tokens: format_tokens(s.total_input_tokens),
            output_tokens: format_tokens(s.total_output_tokens),
            cache_write: format_tokens(s.total_cache_creation_tokens),
            cache_read: format_tokens(s.total_cache_read_tokens),
            est_cost: format!("${:.4}", s.total_cost),
        })
        .collect()
}

fn build_pie_data(records: &[UsageRecord]) -> (Vec<PieSlice>, Vec<PieSlice>) {
    let mut model_cost: HashMap<String, f64> = HashMap::new();
    let mut model_tokens: HashMap<String, f64> = HashMap::new();

    for r in records {
        *model_cost.entry(r.model.clone()).or_default() += estimate_cost(r);
        *model_tokens.entry(r.model.clone()).or_default() +=
            (r.input_tokens + r.output_tokens) as f64;
    }

    let total_cost: f64 = model_cost.values().sum();
    let total_tokens: f64 = model_tokens.values().sum();

    let mut cost_vec: Vec<(String, f64)> = model_cost.into_iter().collect();
    cost_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut token_vec: Vec<(String, f64)> = model_tokens.into_iter().collect();
    token_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let pie_cost: Vec<PieSlice> = cost_vec
        .iter()
        .enumerate()
        .map(|(i, (model, value))| PieSlice {
            model: truncate_model(model, 24),
            value: *value,
            percentage: if total_cost > 0.0 {
                *value / total_cost * 100.0
            } else {
                0.0
            },
            color: MODEL_PALETTE[i % MODEL_PALETTE.len()],
        })
        .collect();

    let pie_tokens: Vec<PieSlice> = token_vec
        .iter()
        .enumerate()
        .map(|(i, (model, value))| PieSlice {
            model: truncate_model(model, 24),
            value: *value,
            percentage: if total_tokens > 0.0 {
                *value / total_tokens * 100.0
            } else {
                0.0
            },
            color: MODEL_PALETTE[i % MODEL_PALETTE.len()],
        })
        .collect();

    (pie_cost, pie_tokens)
}

fn clamp_scroll(scroll: &mut u16, content_len: usize, viewport_height: u16) {
    let max = content_len.saturating_sub(viewport_height as usize) as u16;
    if *scroll > max {
        *scroll = max;
    }
}

pub fn run_tui(records: &[UsageRecord], project_summaries: &[ProjectSummary], range: &TimeRange) -> anyhow::Result<()> {
    if records.is_empty() {
        println!("No usage data found.");
        return Ok(());
    }

    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(records, project_summaries, range);

    loop {
        terminal.draw(|f| draw_ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Right | KeyCode::Char('l') => app.next_tab(),
                KeyCode::Left | KeyCode::Char('h') => app.prev_tab(),
                KeyCode::Char('1') => app.active_tab = TabId::Detail,
                KeyCode::Char('2') => app.active_tab = TabId::DailySummary,
                KeyCode::Char('3') => app.active_tab = TabId::Projects,
                KeyCode::Char('4') => app.active_tab = TabId::Projection,
                KeyCode::Char('5') => app.active_tab = TabId::ModelShare,
                KeyCode::Char('t') => app.chart_mode = app.chart_mode.toggle(),
                KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
                KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
                KeyCode::PageDown => app.scroll_down(10),
                KeyCode::PageUp => app.scroll_up(10),
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn draw_ui(f: &mut ratatui::Frame, app: &mut App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(1),   // content
            Constraint::Length(1), // help bar
        ])
        .split(size);

    draw_tabs(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_help_bar(f, chunks[2]);
}

fn draw_tabs(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = ["Detail", "Daily Summary", "Projects", "Projection", "Model Share"]
        .iter()
        .map(|t| Line::from(*t))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" LLM Usages "))
        .select(app.active_tab.index())
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_content(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let viewport_height = area.height;
    let content_len = app.content_len();

    // Clamp scroll before rendering
    clamp_scroll(app.scroll_mut(), content_len, viewport_height);

    match app.active_tab {
        TabId::Detail => draw_detail_tab(f, app, area),
        TabId::DailySummary => draw_daily_tab(f, app, area),
        TabId::Projects => draw_projects_tab(f, app, area),
        TabId::Projection => draw_projection_tab(f, app, area),
        TabId::ModelShare => draw_model_share_tab(f, app, area),
    }
}

fn draw_detail_tab(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let header_cells = [
        "Provider",
        "Date",
        "Model",
        "Input Tokens",
        "Output Tokens",
        "Cache Write",
        "Cache Read",
        "Est. Cost",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let detail_rows: Vec<Row> = app
        .detail_rows
        .iter()
        .map(|r| {
            let pcolor = provider_color(&r.provider);
            let mcolor = model_color(&r.model_raw);
            Row::new(vec![
                Cell::from(r.provider.clone()).style(Style::default().fg(pcolor)),
                Cell::from(r.date.clone()),
                Cell::from(r.model.clone()).style(Style::default().fg(mcolor)),
                Cell::from(r.input_tokens.clone()),
                Cell::from(r.output_tokens.clone()),
                Cell::from(r.cache_write.clone()),
                Cell::from(r.cache_read.clone()),
                Cell::from(r.est_cost.clone()),
            ])
        })
        .collect();

    // Add summary footer rows
    let mut all_rows = detail_rows;
    if !app.summary_footer.is_empty() {
        all_rows.push(Row::new(vec![Cell::from("")])); // blank separator
        for line in &app.summary_footer {
            all_rows.push(
                Row::new(vec![Cell::from(line.clone())
                    .style(Style::default().add_modifier(Modifier::BOLD))])
            );
        }
    }

    let widths = [
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Length(26),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
    ];

    let table = Table::new(all_rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .row_highlight_style(Style::default())
        .column_spacing(1);

    // Use offset for scrolling
    let mut state = ratatui::widgets::TableState::default();
    state.select(None);
    *state.offset_mut() = app.detail_scroll as usize;

    f.render_stateful_widget(table, area, &mut state);
}

fn draw_daily_tab(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // table
            Constraint::Percentage(60), // chart + legend
        ])
        .split(area);

    // --- Table ---
    let header_cells = [
        "Date",
        "Input Tokens",
        "Output Tokens",
        "Cache Write",
        "Cache Read",
        "Est. Cost",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .daily_rows
        .iter()
        .map(|r| {
            Row::new(vec![
                Cell::from(r.date.clone()),
                Cell::from(r.input_tokens.clone()),
                Cell::from(r.output_tokens.clone()),
                Cell::from(r.cache_write.clone()),
                Cell::from(r.cache_read.clone()),
                Cell::from(r.est_cost.clone()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .column_spacing(1);

    let mut state = ratatui::widgets::TableState::default();
    state.select(None);
    *state.offset_mut() = app.daily_scroll as usize;

    f.render_stateful_widget(table, chunks[0], &mut state);

    // --- Bar Chart + Legend ---
    draw_daily_chart(f, app, chunks[1]);
}

fn draw_daily_chart(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let legend_height = if app.model_legend.len() > 6 { 2 } else { 1 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(legend_height),
        ])
        .split(area);

    let title = format!(" {} (t: toggle) ", app.chart_mode.label());
    let mut chart = BarChart::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .bar_width(3)
        .bar_gap(0)
        .group_gap(2)
        .value_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    for group in &app.chart_groups {
        let bars: Vec<Bar> = group
            .entries
            .iter()
            .map(|e| {
                let value = match app.chart_mode {
                    ChartMode::Cost => (e.cost * 100.0).round() as u64,
                    ChartMode::Tokens => e.total_tokens / 1000,
                };
                let color = MODEL_PALETTE[e.model_index % MODEL_PALETTE.len()];
                Bar::default()
                    .value(value)
                    .style(Style::default().fg(color))
            })
            .collect();

        let bar_group = BarGroup::default()
            .label(Line::from(group.date_short.clone()))
            .bars(&bars);
        chart = chart.data(bar_group);
    }

    f.render_widget(chart, chunks[0]);

    // --- Legend ---
    let mut spans: Vec<Span> = Vec::new();
    for (i, (name, color)) in app.model_legend.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled("\u{25a0} ", Style::default().fg(*color)));
        spans.push(Span::styled(name.clone(), Style::default().fg(Color::Gray)));
    }
    let legend = Paragraph::new(Line::from(spans))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(legend, chunks[1]);
}

fn draw_projects_tab(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let header_cells = [
        "Project",
        "Providers",
        "Input Tokens",
        "Output Tokens",
        "Cache Write",
        "Cache Read",
        "Est. Cost",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .project_rows
        .iter()
        .map(|r| {
            let mut provider_spans: Vec<Span> = Vec::new();
            for (i, p) in r.providers.iter().enumerate() {
                if i > 0 {
                    provider_spans.push(Span::raw(", "));
                }
                provider_spans.push(Span::styled(p.clone(), Style::default().fg(provider_color(p))));
            }
            Row::new(vec![
                Cell::from(r.project.clone()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Cell::from(Line::from(provider_spans)),
                Cell::from(r.input_tokens.clone()),
                Cell::from(r.output_tokens.clone()),
                Cell::from(r.cache_write.clone()),
                Cell::from(r.cache_read.clone()),
                Cell::from(r.est_cost.clone()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(24),
        Constraint::Length(16),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Project Summary (sorted by cost) "))
        .column_spacing(1);

    let mut state = ratatui::widgets::TableState::default();
    state.select(None);
    *state.offset_mut() = app.project_scroll as usize;

    f.render_stateful_widget(table, area, &mut state);
}

fn draw_projection_tab(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let lines: Vec<Line> = app
        .projection_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                Line::from(Span::styled(
                    line.clone(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if line.contains("Projected monthly:") {
                Line::from(Span::styled(
                    line.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(line.clone())
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .scroll((app.projection_scroll, 0));

    f.render_widget(paragraph, area);
}

fn draw_model_share_tab(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let slices = match app.chart_mode {
        ChartMode::Cost => &app.pie_cost,
        ChartMode::Tokens => &app.pie_tokens,
    };

    if slices.is_empty() {
        let msg = Paragraph::new("No data available.")
            .block(Block::default().borders(Borders::ALL).title(" Model Share "));
        f.render_widget(msg, area);
        return;
    }

    // Split: left for pie chart, right for legend
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let pie_area = chunks[0];
    let legend_area = chunks[1];

    // --- Pie Chart using Canvas ---
    let title = format!(" {} (t: toggle) ", app.chart_mode.label());

    // Build cumulative angles for slices
    let mut cumulative_angles: Vec<(f64, f64, Color)> = Vec::new();
    let mut start_angle = 0.0_f64;
    for s in slices {
        let sweep = s.percentage / 100.0 * 360.0;
        cumulative_angles.push((start_angle, start_angle + sweep, s.color));
        start_angle += sweep;
    }

    // Pre-compute points for each slice color
    let canvas_size = 100.0;
    let cx = canvas_size / 2.0;
    let cy = canvas_size / 2.0;
    let radius = canvas_size / 2.0 - 2.0;

    let mut color_points: HashMap<usize, Vec<(f64, f64)>> = HashMap::new();
    let step: f64 = 0.5;
    let mut y: f64 = 0.0;
    while y <= canvas_size {
        let mut x: f64 = 0.0;
        while x <= canvas_size {
            let dx: f64 = x - cx;
            let dy: f64 = y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= radius {
                let angle = dy.atan2(dx).to_degrees();
                let angle = if angle < 0.0 { angle + 360.0 } else { angle };
                for (i, &(start, end, _)) in cumulative_angles.iter().enumerate() {
                    if angle >= start && angle < end {
                        color_points.entry(i).or_default().push((x, y));
                        break;
                    }
                }
            }
            x += step;
        }
        y += step;
    }

    let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .x_bounds([0.0, canvas_size])
        .y_bounds([0.0, canvas_size])
        .paint(move |ctx| {
            for (i, &(_, _, color)) in cumulative_angles.iter().enumerate() {
                if let Some(pts) = color_points.get(&i) {
                    let coords: Vec<(f64, f64)> = pts.clone();
                    ctx.draw(&Points {
                        coords: &coords,
                        color,
                    });
                }
            }
        });

    f.render_widget(canvas, pie_area);

    // --- Legend ---
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        " Model Share Legend",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for s in slices {
        let value_str = match app.chart_mode {
            ChartMode::Cost => format!("${:.4}", s.value),
            ChartMode::Tokens => format_tokens(s.value as u64),
        };
        lines.push(Line::from(vec![
            Span::styled(
                "\u{25a0} ",
                Style::default().fg(s.color),
            ),
            Span::styled(
                format!("{:<24}", s.model),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!(" {:>5.1}%", s.percentage),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", value_str),
                Style::default().fg(Color::Gray),
            ),
        ]));
    }

    let legend = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(legend, legend_area);
}

fn draw_help_bar(f: &mut ratatui::Frame, area: Rect) {
    let help = Line::from(vec![
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(":quit  "),
        Span::styled("\u{2190}/\u{2192}", Style::default().fg(Color::Yellow)),
        Span::raw(":tab  "),
        Span::styled("\u{2191}/\u{2193}", Style::default().fg(Color::Yellow)),
        Span::raw(":scroll  "),
        Span::styled("1-5", Style::default().fg(Color::Yellow)),
        Span::raw(":jump  "),
        Span::styled("PgUp/PgDn", Style::default().fg(Color::Yellow)),
        Span::raw(":page  "),
        Span::styled("t", Style::default().fg(Color::Yellow)),
        Span::raw(":chart"),
    ]);

    let paragraph = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(paragraph, area);
}
