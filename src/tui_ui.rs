use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, Widget,
    },
    Frame,
};

use crate::format::{
    format_cost, format_duration, format_number, format_relative, short_model, truncate,
};
use crate::models::DataSource;
use crate::theme::ThemeColors;
use crate::tui_app::{App, InputMode, SortColumn, View};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let tc = app.theme.colors();
    let size = frame.area();

    // Clear background
    let bg_block = Block::default().style(Style::default().bg(tc.bg));
    frame.render_widget(bg_block, size);

    if app.loading {
        draw_loading(frame, app, &tc, size);
        return;
    }

    // Main layout: header + content + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),   // content
            Constraint::Length(1), // footer
        ])
        .split(size);

    draw_header(frame, app, &tc, chunks[0]);

    match app.view {
        View::Dashboard => draw_dashboard(frame, app, &tc, chunks[1]),
        View::ProjectList => draw_project_list(frame, app, &tc, chunks[1]),
        View::ProjectDetail => draw_project_detail(frame, app, &tc, chunks[1]),
        View::SessionDetail => draw_session_detail(frame, app, &tc, chunks[1]),
    }

    draw_footer(frame, app, &tc, chunks[2]);
}

fn draw_loading(frame: &mut Frame, app: &App, tc: &ThemeColors, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(5),
            Constraint::Percentage(40),
        ])
        .split(area);

    let center = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(chunks[1]);

    let loading = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Loading...",
            Style::default().fg(tc.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            &app.loading_status,
            Style::default().fg(tc.muted),
        )),
    ])
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(loading, center[1]);
}

fn draw_header(frame: &mut Frame, app: &App, tc: &ThemeColors, area: Rect) {
    let m = &app.metrics;
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" ct ", Style::default().fg(tc.accent).add_modifier(Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(tc.border)),
        Span::styled(
            format!("{} projects", m.total_projects),
            Style::default().fg(tc.fg),
        ),
        Span::styled(" │ ", Style::default().fg(tc.border)),
        Span::styled(
            format!("{} sessions", m.total_sessions),
            Style::default().fg(tc.fg),
        ),
        Span::styled(" │ ", Style::default().fg(tc.border)),
        Span::styled(
            format!("{} msgs", format_number(m.total_messages as u64)),
            Style::default().fg(tc.fg),
        ),
        Span::styled(" │ ", Style::default().fg(tc.border)),
        Span::styled(
            format!("{} tokens", format_number(m.total_tokens.total())),
            Style::default().fg(tc.token_input),
        ),
        Span::styled(" │ ", Style::default().fg(tc.border)),
        Span::styled(
            format_cost(m.total_cost),
            Style::default().fg(tc.success).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(tc.border)),
        Span::styled(
            format!("Theme: {}", app.theme),
            Style::default().fg(tc.muted),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(header, area);
}

fn draw_footer(frame: &mut Frame, app: &App, tc: &ThemeColors, area: Rect) {
    let keys = match app.view {
        View::Dashboard => "Enter: Projects │ t: Theme │ q: Quit",
        View::ProjectList => match app.input_mode {
            InputMode::Search => "Type to filter │ Enter: Confirm │ Esc: Cancel",
            InputMode::Normal => "j/k: Navigate │ Enter: Detail │ /: Search │ s: Sort │ t: Theme │ q: Quit",
        },
        View::ProjectDetail => "j/k: Navigate │ Enter: Session │ Esc: Back │ t: Theme │ q: Quit",
        View::SessionDetail => {
            "j/k: Scroll │ u/d: Page │ g/G: Top/Bottom │ Esc: Back │ t: Theme │ q: Quit"
        }
    };

    let footer = Paragraph::new(Span::styled(
        format!(" {}", keys),
        Style::default().fg(tc.muted),
    ));
    frame.render_widget(footer, area);
}

/// Render a single unicode horizontal bar line: `label ████████ value`
fn unicode_bar_line<'a>(
    label: &str,
    value: u64,
    max_value: u64,
    max_bar_width: u16,
    label_width: usize,
    color: Color,
    tc: &ThemeColors,
) -> Line<'a> {
    let bar_len = if max_value > 0 {
        ((value as f64 / max_value as f64) * max_bar_width as f64).round() as usize
    } else {
        0
    };
    let bar_len = bar_len.max(if value > 0 { 1 } else { 0 });
    let bar: String = "█".repeat(bar_len);

    Line::from(vec![
        Span::styled(
            format!("{:>width$} ", label, width = label_width),
            Style::default().fg(tc.fg),
        ),
        Span::styled(bar, Style::default().fg(color)),
        Span::styled(
            format!(" {}", format_number(value)),
            Style::default().fg(tc.muted),
        ),
    ])
}

/// Custom stacked bar chart widget for the activity timeline
struct StackedBarChart<'a> {
    timeline: &'a [crate::models::TimelineEntry],
    claude_color: Color,
    cursor_color: Color,
    axis_color: Color,
    block: Option<Block<'a>>,
}

impl<'a> Widget for StackedBarChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chart_area = if let Some(block) = &self.block {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        } else {
            area
        };

        if chart_area.width < 10 || chart_area.height < 4 || self.timeline.is_empty() {
            return;
        }

        // Reserve space: left for Y-axis labels, bottom for X-axis labels
        let y_label_width: u16 = 4;
        let x_label_height: u16 = 1;
        let bar_area_x = chart_area.x + y_label_width;
        let bar_area_y = chart_area.y;
        let bar_area_w = chart_area.width.saturating_sub(y_label_width);
        let bar_area_h = chart_area.height.saturating_sub(x_label_height);

        if bar_area_w == 0 || bar_area_h == 0 {
            return;
        }

        // Calculate how many bars we can fit (each bar is 1 char wide, with optional gaps)
        let total_entries = self.timeline.len();
        let available_cols = bar_area_w as usize;

        // If more entries than columns, sample/aggregate; if fewer, use 1 col per entry
        let (bar_data, date_labels): (Vec<(u64, u64)>, Vec<String>) = if total_entries <= available_cols {
            // One bar per entry, no gap needed
            let data: Vec<(u64, u64)> = self.timeline.iter()
                .map(|t| (t.claude_sessions, t.cursor_sessions))
                .collect();
            let labels: Vec<String> = self.timeline.iter()
                .map(|t| t.date.clone())
                .collect();
            (data, labels)
        } else {
            // Aggregate entries into buckets
            let bucket_size = (total_entries + available_cols - 1) / available_cols;
            let mut data = Vec::new();
            let mut labels = Vec::new();
            for chunk in self.timeline.chunks(bucket_size) {
                let claude: u64 = chunk.iter().map(|t| t.claude_sessions).sum();
                let cursor: u64 = chunk.iter().map(|t| t.cursor_sessions).sum();
                data.push((claude, cursor));
                labels.push(chunk[0].date.clone());
            }
            // Recompute max for aggregated data
            (data, labels)
        };

        let agg_max = bar_data.iter().map(|(c, r)| c + r).max().unwrap_or(1).max(1);
        let num_bars = bar_data.len();

        // Y-axis labels (draw a few tick marks)
        let y_ticks = 4usize.min(bar_area_h as usize);
        for i in 0..y_ticks {
            let frac = i as f64 / (y_ticks - 1).max(1) as f64;
            let val = (agg_max as f64 * frac) as u64;
            let row = bar_area_y + bar_area_h - 1 - ((frac * (bar_area_h - 1) as f64) as u16);
            let label = format!("{:>3}", val);
            let x = chart_area.x;
            for (j, ch) in label.chars().enumerate() {
                let col = x + j as u16;
                if col < bar_area_x && row < bar_area_y + bar_area_h {
                    buf[(col, row)].set_char(ch).set_style(Style::default().fg(self.axis_color));
                }
            }
        }

        // Draw bars
        let cols_per_bar = if num_bars > 0 { available_cols / num_bars } else { 1 };
        let cols_per_bar = cols_per_bar.max(1);

        for (i, &(claude, cursor)) in bar_data.iter().enumerate() {
            let total = claude + cursor;
            if total == 0 {
                continue;
            }

            let bar_height_f = (total as f64 / agg_max as f64) * bar_area_h as f64;
            let bar_height = bar_height_f.round() as u16;
            let bar_height = bar_height.max(if total > 0 { 1 } else { 0 });

            let claude_height_f = (claude as f64 / agg_max as f64) * bar_area_h as f64;
            let claude_height = claude_height_f.round() as u16;
            let claude_height = claude_height.max(if claude > 0 { 1 } else { 0 }).min(bar_height);
            let cursor_height = bar_height.saturating_sub(claude_height);

            let x_start = bar_area_x + (i * cols_per_bar) as u16;
            let bar_width = if cols_per_bar > 1 { (cols_per_bar - 0) as u16 } else { 1 };

            // Draw from bottom up: Claude first (bottom), then Cursor (top)
            for dy in 0..bar_height {
                let row = bar_area_y + bar_area_h - 1 - dy;
                let color = if dy < claude_height {
                    self.claude_color
                } else if dy < claude_height + cursor_height {
                    self.cursor_color
                } else {
                    self.claude_color
                };
                for dx in 0..bar_width {
                    let col = x_start + dx;
                    if col < bar_area_x + bar_area_w && row >= bar_area_y {
                        buf[(col, row)].set_char('█').set_style(Style::default().fg(color));
                    }
                }
            }
        }

        // X-axis date labels
        let label_row = bar_area_y + bar_area_h;
        if label_row < chart_area.y + chart_area.height {
            // Show ~5 evenly-spaced date labels
            let num_labels = 5usize.min(num_bars);
            if num_labels > 0 && num_bars > 0 {
                for li in 0..num_labels {
                    let idx = if num_labels == 1 {
                        0
                    } else {
                        li * (num_bars - 1) / (num_labels - 1)
                    };
                    let x_pos = bar_area_x + (idx * cols_per_bar) as u16;
                    // Show MM-DD portion of date
                    let label = if date_labels[idx].len() >= 10 {
                        &date_labels[idx][5..10] // MM-DD
                    } else {
                        &date_labels[idx]
                    };
                    for (j, ch) in label.chars().enumerate() {
                        let col = x_pos + j as u16;
                        if col < bar_area_x + bar_area_w {
                            buf[(col, label_row)].set_char(ch).set_style(Style::default().fg(self.axis_color));
                        }
                    }
                }
            }
        }
    }
}

fn draw_dashboard(frame: &mut Frame, app: &App, tc: &ThemeColors, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(6),  // stats cards
            Constraint::Length(10), // token breakdown + tool usage
            Constraint::Min(4),    // activity sparkline
        ])
        .split(area);

    // Stats cards row
    let card_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(16),
            Constraint::Percentage(16),
            Constraint::Percentage(16),
            Constraint::Percentage(16),
            Constraint::Percentage(16),
            Constraint::Percentage(20),
        ])
        .split(chunks[0]);

    let m = &app.metrics;
    draw_stat_card(frame, tc, card_chunks[0], "Projects", &m.total_projects.to_string(), tc.accent);
    draw_stat_card(frame, tc, card_chunks[1], "Sessions", &m.total_sessions.to_string(), tc.accent);
    draw_stat_card(frame, tc, card_chunks[2], "Messages", &format_number(m.total_messages as u64), tc.accent);
    draw_stat_card(
        frame,
        tc,
        card_chunks[3],
        "Lines +/-",
        &format!(
            "{}/{}",
            format_number(m.total_lines_added),
            format_number(m.total_lines_removed)
        ),
        tc.success,
    );
    draw_stat_card(
        frame,
        tc,
        card_chunks[4],
        "Tokens",
        &format_number(m.total_tokens.total()),
        tc.token_input,
    );
    draw_stat_card(
        frame,
        tc,
        card_chunks[5],
        "Est. Cost",
        &format_cost(m.total_cost),
        tc.success,
    );

    // Token breakdown + tool usage
    let mid_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    // Token breakdown with proportional bars
    let token_values = [
        ("Input", m.total_tokens.input, tc.token_input),
        ("Output", m.total_tokens.output, tc.token_output),
        ("Cache R", m.total_tokens.cache_read, tc.token_cache),
        ("Cache W", m.total_tokens.cache_creation, tc.token_cache),
    ];
    let token_max = token_values.iter().map(|(_, v, _)| *v).max().unwrap_or(1);
    let bar_width = mid_chunks[0].width.saturating_sub(22); // label + value space

    let mut token_lines: Vec<Line> = token_values
        .iter()
        .map(|(label, val, color)| {
            unicode_bar_line(label, *val, token_max, bar_width, 8, *color, tc)
        })
        .collect();

    token_lines.push(Line::from(vec![
        Span::styled("  Human: ", Style::default().fg(tc.muted)),
        Span::styled(
            format!(
                "{} lines, {} words",
                format_number(m.human_lines),
                format_number(m.human_words)
            ),
            Style::default().fg(tc.fg),
        ),
    ]));

    let token_block = Paragraph::new(token_lines).block(
        Block::default()
            .title(Span::styled(" Tokens ", Style::default().fg(tc.title)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(token_block, mid_chunks[0]);

    // Tool usage — unicode horizontal bars
    let mut sorted_tools: Vec<(&String, &u64)> = m.tool_usage.iter().collect();
    sorted_tools.sort_by(|a, b| b.1.cmp(a.1));
    sorted_tools.truncate(8);

    let tool_max = sorted_tools.first().map(|(_, &v)| v).unwrap_or(1);
    let tool_bar_width = mid_chunks[1].width.saturating_sub(22);

    let tool_lines: Vec<Line> = sorted_tools
        .iter()
        .enumerate()
        .map(|(i, (name, &count))| {
            let color = if i % 2 == 0 { tc.bar } else { tc.bar_alt };
            unicode_bar_line(&truncate(name, 10), count, tool_max, tool_bar_width, 10, color, tc)
        })
        .collect();

    let tool_block = Paragraph::new(tool_lines).block(
        Block::default()
            .title(Span::styled(" Tool Usage ", Style::default().fg(tc.title)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(tool_block, mid_chunks[1]);

    // Activity stacked bar chart
    let peak_info = app.metrics.timeline.iter()
        .max_by_key(|t| t.sessions)
        .map(|t| format!("Peak: {} ({} sessions)", t.date, t.sessions))
        .unwrap_or_default();

    let has_cursor = app.metrics.timeline.iter().any(|t| t.cursor_sessions > 0);
    let has_claude = app.metrics.timeline.iter().any(|t| t.claude_sessions > 0);

    let mut title_spans = vec![
        Span::styled(" Activity (sessions/day) ", Style::default().fg(tc.title)),
    ];
    if !peak_info.is_empty() {
        title_spans.push(Span::styled(
            format!(" {} ", peak_info),
            Style::default().fg(tc.muted),
        ));
    }
    // Legend
    if has_claude && has_cursor {
        title_spans.push(Span::styled(" │ ", Style::default().fg(tc.border)));
        title_spans.push(Span::styled("█", Style::default().fg(tc.claude_badge)));
        title_spans.push(Span::styled(" Claude ", Style::default().fg(tc.muted)));
        title_spans.push(Span::styled("█", Style::default().fg(tc.cursor_badge)));
        title_spans.push(Span::styled(" Cursor ", Style::default().fg(tc.muted)));
    }

    let chart = StackedBarChart {
        timeline: &app.metrics.timeline,
        claude_color: tc.claude_badge,
        cursor_color: tc.cursor_badge,
        axis_color: tc.muted,
        block: Some(
            Block::default()
                .title(Line::from(title_spans))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tc.border)),
        ),
    };
    frame.render_widget(chart, chunks[2]);
}

fn draw_stat_card(frame: &mut Frame, tc: &ThemeColors, area: Rect, label: &str, value: &str, color: Color) {
    let card = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(label, Style::default().fg(tc.muted))),
    ])
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(card, area);
}

fn source_badge<'a>(sources: &[DataSource], tc: &ThemeColors) -> Span<'a> {
    if sources.contains(&DataSource::Claude) && sources.contains(&DataSource::Cursor) {
        Span::styled(" Both ", Style::default().fg(tc.accent).add_modifier(Modifier::BOLD))
    } else if sources.contains(&DataSource::Cursor) {
        Span::styled(" Cursor ", Style::default().fg(tc.cursor_badge).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" Claude ", Style::default().fg(tc.claude_badge).add_modifier(Modifier::BOLD))
    }
}

fn draw_project_list(frame: &mut Frame, app: &mut App, tc: &ThemeColors, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search bar
    let search_line = if app.input_mode == InputMode::Search {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(tc.accent)),
            Span::styled(&app.search_query, Style::default().fg(tc.fg)),
            Span::styled("█", Style::default().fg(tc.accent)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!(" Sort: {} ", app.sort_column.label()),
                Style::default().fg(tc.muted),
            ),
            if !app.search_query.is_empty() {
                Span::styled(
                    format!("│ Filter: {} ", &app.search_query),
                    Style::default().fg(tc.accent),
                )
            } else {
                Span::raw("")
            },
            Span::styled(
                format!(
                    "│ {}/{} projects",
                    app.filtered_projects.len(),
                    app.projects.len()
                ),
                Style::default().fg(tc.muted),
            ),
        ])
    };

    let search_bar = Paragraph::new(search_line).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(search_bar, chunks[0]);

    // Build sort indicator
    let sort_indicator = |col: SortColumn| -> &str {
        if app.sort_column == col { " ▼" } else { "" }
    };

    // Project table
    let header_cells = [
        format!("Project{}", sort_indicator(SortColumn::Name)),
        "Source".to_string(),
        format!("Sess{}", sort_indicator(SortColumn::Sessions)),
        format!("Msgs{}", sort_indicator(SortColumn::Messages)),
        format!("Tokens{}", sort_indicator(SortColumn::Tokens)),
        format!("Lines +/-{}", sort_indicator(SortColumn::Lines)),
        format!("Cost{}", sort_indicator(SortColumn::Cost)),
        "Model".to_string(),
        format!("Last Active{}", sort_indicator(SortColumn::LastActive)),
    ];
    let header = Row::new(
        header_cells
            .iter()
            .map(|h| Cell::from(h.as_str()).style(Style::default().fg(tc.accent))),
    )
    .height(1);

    let rows: Vec<Row> = app
        .filtered_projects
        .iter()
        .map(|&idx| {
            let p = &app.projects[idx];
            let model_color = tc.model_color(&p.model);
            let source_label = source_label_str(&p.sources);
            let source_color = source_color(&p.sources, tc);
            Row::new(vec![
                Cell::from(truncate(&p.name, 28)).style(Style::default().fg(tc.fg)),
                Cell::from(source_label).style(Style::default().fg(source_color)),
                Cell::from(p.session_count.to_string()).style(Style::default().fg(tc.fg)),
                Cell::from(p.message_count.to_string()).style(Style::default().fg(tc.fg)),
                Cell::from(format_number(p.total_tokens.total()))
                    .style(Style::default().fg(tc.token_input)),
                Cell::from(format!(
                    "+{}/-{}",
                    format_number(p.lines_added),
                    format_number(p.lines_removed)
                ))
                .style(Style::default().fg(tc.success)),
                Cell::from(format_cost(p.cost)).style(Style::default().fg(tc.success)),
                Cell::from(short_model(&p.model)).style(Style::default().fg(model_color)),
                Cell::from(format_relative(&p.last_active)).style(Style::default().fg(tc.muted)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(22),
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(9),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Percentage(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(Span::styled(" Projects ", Style::default().fg(tc.title)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    )
    .row_highlight_style(
        Style::default()
            .bg(tc.highlight_bg)
            .fg(tc.highlight_fg)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(table, chunks[1], &mut app.project_table_state);

    // Scrollbar
    let content_len = app.filtered_projects.len();
    if content_len > 0 {
        let mut scrollbar_state = ScrollbarState::new(content_len).position(app.selected_project);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(tc.muted));
        frame.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
    }
}

fn source_label_str(sources: &[DataSource]) -> String {
    if sources.contains(&DataSource::Claude) && sources.contains(&DataSource::Cursor) {
        "Both".to_string()
    } else if sources.contains(&DataSource::Cursor) {
        "Cursor".to_string()
    } else {
        "Claude".to_string()
    }
}

fn source_color(sources: &[DataSource], tc: &ThemeColors) -> Color {
    if sources.contains(&DataSource::Claude) && sources.contains(&DataSource::Cursor) {
        tc.accent
    } else if sources.contains(&DataSource::Cursor) {
        tc.cursor_badge
    } else {
        tc.claude_badge
    }
}

fn draw_project_detail(frame: &mut Frame, app: &mut App, tc: &ThemeColors, area: Rect) {
    let project = match app.current_project() {
        Some(p) => p.clone(),
        None => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(5), // project info
            Constraint::Min(0),   // session table
        ])
        .split(area);

    // Project info
    let model_color = tc.model_color(&project.model);
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(&project.name, Style::default().fg(tc.title).add_modifier(Modifier::BOLD)),
            Span::styled("  ", Style::default()),
            Span::styled(short_model(&project.model), Style::default().fg(model_color)),
            Span::styled("  ", Style::default()),
            source_badge(&project.sources, tc),
        ]),
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(tc.muted)),
            Span::styled(&project.path, Style::default().fg(tc.fg)),
        ]),
        Line::from(vec![
            Span::styled(
                format!(
                    "{} sessions │ {} messages │ {} tokens │ +{}/−{} lines │ {}",
                    project.session_count,
                    project.message_count,
                    format_number(project.total_tokens.total()),
                    format_number(project.lines_added),
                    format_number(project.lines_removed),
                    format_cost(project.cost),
                ),
                Style::default().fg(tc.fg),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(info, chunks[0]);

    // Session table
    let header = Row::new(vec![
        Cell::from("First Prompt").style(Style::default().fg(tc.accent)),
        Cell::from("Messages").style(Style::default().fg(tc.accent)),
        Cell::from("Tokens").style(Style::default().fg(tc.accent)),
        Cell::from("Duration").style(Style::default().fg(tc.accent)),
        Cell::from("Lines +/-").style(Style::default().fg(tc.accent)),
        Cell::from("Model").style(Style::default().fg(tc.accent)),
        Cell::from("Started").style(Style::default().fg(tc.accent)),
    ]);

    let rows: Vec<Row> = project
        .sessions
        .iter()
        .map(|s| {
            let mc = tc.model_color(&s.model);
            Row::new(vec![
                Cell::from(Line::from(style_xml_content(&s.first_prompt, tc.fg, tc.xml_tag))),
                Cell::from(s.messages.len().to_string()).style(Style::default().fg(tc.fg)),
                Cell::from(format_number(s.total_tokens.total()))
                    .style(Style::default().fg(tc.token_input)),
                Cell::from(format_duration(s.duration_ms)).style(Style::default().fg(tc.fg)),
                Cell::from(format!(
                    "+{}/−{}",
                    format_number(s.lines_added),
                    format_number(s.lines_removed)
                ))
                .style(Style::default().fg(tc.success)),
                Cell::from(short_model(&s.model)).style(Style::default().fg(mc)),
                Cell::from(format_relative(&s.started_at)).style(Style::default().fg(tc.muted)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(30),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(Span::styled(" Sessions ", Style::default().fg(tc.title)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    )
    .row_highlight_style(
        Style::default()
            .bg(tc.highlight_bg)
            .fg(tc.highlight_fg)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(table, chunks[1], &mut app.session_table_state);
}

fn draw_session_detail(frame: &mut Frame, app: &mut App, tc: &ThemeColors, area: Rect) {
    let project = match app.current_project() {
        Some(p) => p.clone(),
        None => return,
    };
    let session = match project.sessions.get(app.selected_session) {
        Some(s) => s.clone(),
        None => return,
    };

    // Layout: compact info at top, then messages + files side by side
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // compact session info
            Constraint::Min(4),   // messages + files
        ])
        .split(area);

    draw_session_info_compact(frame, &session, tc, chunks[0]);

    // Split bottom area: messages (left) + files (right)
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[1]);

    draw_message_thread(frame, app, &session, tc, bottom_chunks[0]);
    draw_files_panel(frame, &session, tc, bottom_chunks[1]);
}

fn draw_session_info_compact(
    frame: &mut Frame,
    session: &crate::models::ParsedSession,
    tc: &ThemeColors,
    area: Rect,
) {
    let model_color = tc.model_color(&session.model);
    let cost = crate::format::estimate_cost(
        &session.model,
        session.total_tokens.input,
        session.total_tokens.output,
        session.total_tokens.cache_read,
    );
    let source_color = match session.source {
        DataSource::Cursor => tc.cursor_badge,
        DataSource::Claude => tc.claude_badge,
    };

    let prompt_spans = style_xml_content(&session.first_prompt, tc.fg, tc.xml_tag);
    let info = Paragraph::new(vec![
        Line::from(prompt_spans),
        Line::from(vec![
            Span::styled(short_model(&session.model), Style::default().fg(model_color)),
            Span::styled("  ", Style::default()),
            Span::styled(session.source.label(), Style::default().fg(source_color).add_modifier(Modifier::BOLD)),
            Span::styled("  │  ", Style::default().fg(tc.border)),
            Span::styled(format!("{} msgs", session.messages.len()), Style::default().fg(tc.fg)),
            Span::styled("  │  ", Style::default().fg(tc.border)),
            Span::styled(format_duration(session.duration_ms), Style::default().fg(tc.fg)),
            Span::styled("  │  ", Style::default().fg(tc.border)),
            Span::styled(format!("+{}", format_number(session.lines_added)), Style::default().fg(tc.success)),
            Span::styled("/", Style::default().fg(tc.muted)),
            Span::styled(format!("−{}", format_number(session.lines_removed)), Style::default().fg(tc.danger)),
            Span::styled("  │  ", Style::default().fg(tc.border)),
            Span::styled(format_cost(cost), Style::default().fg(tc.success)),
            Span::styled("  │  ", Style::default().fg(tc.border)),
            Span::styled(
                format!(
                    "{}in/{}out/{}cache",
                    format_number(session.total_tokens.input),
                    format_number(session.total_tokens.output),
                    format_number(session.total_tokens.cache_read),
                ),
                Style::default().fg(tc.muted),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(info, area);
}

fn draw_files_panel(
    frame: &mut Frame,
    session: &crate::models::ParsedSession,
    tc: &ThemeColors,
    area: Rect,
) {
    let mut sorted_files: Vec<(&String, &crate::models::FileContribution)> =
        session.file_contributions.iter().collect();
    sorted_files.sort_by(|a, b| (b.1.added + b.1.removed).cmp(&(a.1.added + a.1.removed)));

    let file_rows: Vec<Row> = sorted_files
        .iter()
        .map(|(path, fc)| {
            let short_path = path.split('/').rev().take(3).collect::<Vec<_>>();
            let display_path = short_path.into_iter().rev().collect::<Vec<_>>().join("/");
            Row::new(vec![
                Cell::from(display_path).style(Style::default().fg(tc.fg)),
                Cell::from(format!("+{}", fc.added)).style(Style::default().fg(tc.success)),
                Cell::from(format!("−{}", fc.removed)).style(Style::default().fg(tc.danger)),
            ])
        })
        .collect();

    let file_header = Row::new(vec![
        Cell::from("File").style(Style::default().fg(tc.accent)),
        Cell::from("+").style(Style::default().fg(tc.accent)),
        Cell::from("−").style(Style::default().fg(tc.accent)),
    ]);

    let total_files = sorted_files.len();
    let file_table = Table::new(
        file_rows,
        [
            Constraint::Min(10),
            Constraint::Length(6),
            Constraint::Length(6),
        ],
    )
    .header(file_header)
    .block(
        Block::default()
            .title(Span::styled(
                format!(" Files ({}) ", total_files),
                Style::default().fg(tc.title),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(file_table, area);
}

/// Remove XML tag markup but keep the content between tags, colored in `tag_color`.
/// `<command-message>hello</command-message> world` →
///   [Span("hello", purple), Span(" world", normal)]
fn style_xml_content<'a>(s: &str, text_color: Color, tag_color: Color) -> Vec<Span<'a>> {
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut current_text = String::new();
    let mut in_tag_content = false; // true when we're between <tag> and </tag>
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Scan ahead for closing '>'
            let mut tag = String::from('<');
            let mut found_close = false;
            if chars.peek().map_or(true, |c| c.is_whitespace()) {
                current_text.push(ch);
                continue;
            }
            for next in chars.by_ref() {
                tag.push(next);
                if next == '>' {
                    found_close = true;
                    break;
                }
                if tag.len() > 200 {
                    break;
                }
            }
            if found_close {
                // Flush accumulated text before this tag
                if !current_text.is_empty() {
                    let color = if in_tag_content { tag_color } else { text_color };
                    spans.push(Span::styled(current_text.clone(), Style::default().fg(color)));
                    current_text.clear();
                }
                // Check if this is an opening or closing tag
                let is_closing = tag.starts_with("</");
                if is_closing {
                    in_tag_content = false;
                } else {
                    in_tag_content = true;
                }
                // Tag markup itself is not rendered
            } else {
                current_text.push_str(&tag);
            }
        } else {
            current_text.push(ch);
        }
    }

    // Flush remaining text
    if !current_text.is_empty() {
        let color = if in_tag_content { tag_color } else { text_color };
        spans.push(Span::styled(current_text, Style::default().fg(color)));
    }

    if spans.is_empty() {
        spans.push(Span::styled("", Style::default()));
    }

    spans
}



fn draw_message_thread(
    frame: &mut Frame,
    app: &mut App,
    session: &crate::models::ParsedSession,
    tc: &ThemeColors,
    area: Rect,
) {
    let messages = &session.messages;
    if messages.is_empty() {
        let empty = Paragraph::new("No messages in this session")
            .style(Style::default().fg(tc.muted))
            .block(
                Block::default()
                    .title(Span::styled(" Messages ", Style::default().fg(tc.title)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(tc.border)),
            );
        frame.render_widget(empty, area);
        return;
    }

    // Available height inside the block (borders take 2 lines)
    let inner_height = area.height.saturating_sub(2) as usize;
    let max_lines_per_msg: usize = 8;

    // Build display lines for all messages
    let mut all_lines: Vec<Line> = Vec::new();
    // Track which line index each message starts at for scrolling
    let mut msg_line_offsets: Vec<usize> = Vec::new();

    for msg in messages {
        msg_line_offsets.push(all_lines.len());

        let (role_label, role_color) = if msg.role == "user" {
            ("You", tc.accent)
        } else {
            ("Assistant", tc.token_output)
        };

        let ts = format_relative(&msg.timestamp);

        // Role header
        all_lines.push(Line::from(vec![
            Span::styled(
                format!("── {} ", role_label),
                Style::default().fg(role_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(ts, Style::default().fg(tc.muted)),
            Span::styled(
                " ──────────────────────────────────────────",
                Style::default().fg(tc.border),
            ),
        ]));

        // Content lines
        let content = &msg.content;
        if content.is_empty() {
            all_lines.push(Line::from(Span::styled(
                "  (no text content)",
                Style::default().fg(tc.muted),
            )));
        } else {
            let content_lines: Vec<&str> = content.lines().collect();
            let total = content_lines.len();
            let show = total.min(max_lines_per_msg);
            for line in content_lines.iter().take(show) {
                let display = if line.len() > 200 {
                    format!("  {}...", &line[..197])
                } else {
                    format!("  {}", line)
                };
                let spans = style_xml_content(&display, tc.fg, tc.xml_tag);
                all_lines.push(Line::from(spans));
            }
            if total > max_lines_per_msg {
                all_lines.push(Line::from(Span::styled(
                    format!("  ... ({} more lines)", total - max_lines_per_msg),
                    Style::default().fg(tc.muted),
                )));
            }
        }

        // Blank separator line
        all_lines.push(Line::from(""));
    }

    // Calculate scroll offset in lines based on message_scroll
    let scroll_line = if app.message_scroll < msg_line_offsets.len() {
        msg_line_offsets[app.message_scroll]
    } else {
        0
    };

    // Clamp scroll so we don't go past the end
    let max_scroll_line = all_lines.len().saturating_sub(inner_height);
    let scroll_line = scroll_line.min(max_scroll_line);

    let visible_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(scroll_line)
        .take(inner_height)
        .collect();

    let msg_title = format!(
        " Messages ({}/{}) ",
        (app.message_scroll + 1).min(messages.len()),
        messages.len()
    );

    let msg_block = Paragraph::new(visible_lines).block(
        Block::default()
            .title(Span::styled(msg_title, Style::default().fg(tc.title)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(msg_block, area);

    // Scrollbar
    if messages.len() > 1 {
        let mut scrollbar_state = ScrollbarState::new(messages.len()).position(app.message_scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(tc.muted));
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
