use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, Paragraph, Row, Table, Tabs,
    },
};

fn axis_labels<'a>(items: Vec<&'a str>) -> Vec<Line<'a>> {
    items.into_iter().map(Line::from).collect()
}

use crate::app::{App, SortBy, Tab, HISTORY_LEN};

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Root layout: tabs + content + help bar
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Min(0),    // content
            Constraint::Length(1), // help
        ])
        .split(area);

    draw_tabs(f, app, root[0]);

    match app.tab {
        Tab::Overview => draw_overview(f, app, root[1]),
        Tab::Processes => draw_processes(f, app, root[1]),
    }

    draw_help(f, app, root[2]);
}

// ── Tabs ──────────────────────────────────────────────────────────────────────

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = vec![
        Line::from(Span::styled("  Overview  ", Style::default())),
        Line::from(Span::styled("  Processes  ", Style::default())),
    ];
    let selected = match app.tab {
        Tab::Overview => 0,
        Tab::Processes => 1,
    };
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" tui-monitor "))
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

// ── Help bar ─────────────────────────────────────────────────────────────────

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(" Tab", Style::default().fg(Color::Yellow)),
        Span::raw(": switch  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(": quit"),
    ];
    if app.tab == Tab::Processes {
        spans.extend([
            Span::raw("  "),
            Span::styled("↑↓/jk", Style::default().fg(Color::Yellow)),
            Span::raw(": scroll  "),
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::raw(": sort ("),
            Span::styled(
                match app.sort_by {
                    SortBy::Cpu => "CPU",
                    SortBy::Memory => "MEM",
                    SortBy::Pid => "PID",
                    SortBy::Name => "NAME",
                },
                Style::default().fg(Color::Green),
            ),
            Span::raw(")"),
        ]);
    }
    let p = Paragraph::new(Line::from(spans))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(p, area);
}

// ── Overview ──────────────────────────────────────────────────────────────────

fn draw_overview(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // CPU
            Constraint::Percentage(30), // Memory
            Constraint::Percentage(30), // Disk + Net
        ])
        .split(area);

    draw_cpu(f, app, rows[0]);
    draw_memory(f, app, rows[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[2]);
    draw_disk(f, app, bottom[0]);
    draw_network(f, app, bottom[1]);
}

// ── CPU ───────────────────────────────────────────────────────────────────────

fn draw_cpu(f: &mut Frame, app: &App, area: Rect) {
    let core_count = app.cpu_usage.len();

    // Left: per-core gauges | Right: aggregate sparkline
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Per-core gauges
    let gauge_block = Block::default().borders(Borders::ALL).title(format!(
        " CPU  avg {:.1}% ",
        app.avg_cpu()
    ));
    let inner = gauge_block.inner(cols[0]);
    f.render_widget(gauge_block, cols[0]);

    if core_count == 0 {
        return;
    }
    let constraints: Vec<Constraint> = (0..core_count)
        .map(|_| Constraint::Length(1))
        .collect();
    let gauge_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, usage) in app.cpu_usage.iter().enumerate() {
        if i >= gauge_rows.len() {
            break;
        }
        let pct = (*usage as u16).min(100);
        let color = usage_color(*usage);
        let gauge = Gauge::default()
            .label(format!("c{i:<2} {usage:>5.1}%"))
            .ratio(*usage as f64 / 100.0)
            .style(Style::default().fg(color))
            .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
            .use_unicode(true);
        let _ = pct;
        f.render_widget(gauge, gauge_rows[i]);
    }

    // Aggregate history chart
    let avg_data: Vec<(f64, f64)> = app.cpu_history[0]
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let avg: f64 = app
                .cpu_history
                .iter()
                .map(|h| h.get(i).copied().unwrap_or(0.0))
                .sum::<f64>()
                / core_count as f64;
            (i as f64, avg)
        })
        .collect();

    let dataset = Dataset::default()
        .name("avg%")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&avg_data);

    let chart = Chart::new(vec![dataset])
        .block(Block::default().borders(Borders::ALL).title(" CPU History "))
        .x_axis(
            Axis::default()
                .bounds([0.0, HISTORY_LEN as f64])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(axis_labels(vec!["0", "50", "100"]))
                .style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(chart, cols[1]);
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn draw_memory(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Gauges
    let mem_pct = if app.mem_total > 0 {
        app.mem_used as f64 / app.mem_total as f64
    } else {
        0.0
    };
    let swap_pct = if app.swap_total > 0 {
        app.swap_used as f64 / app.swap_total as f64
    } else {
        0.0
    };

    let gauge_block = Block::default().borders(Borders::ALL).title(format!(
        " Memory  {}/{} ",
        fmt_bytes(app.mem_used),
        fmt_bytes(app.mem_total)
    ));
    let inner = gauge_block.inner(cols[0]);
    f.render_widget(gauge_block, cols[0]);

    let gauge_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(inner);

    let mem_gauge = Gauge::default()
        .label(format!("RAM  {:.1}%", mem_pct * 100.0))
        .ratio(mem_pct)
        .style(Style::default().fg(usage_color((mem_pct * 100.0) as f32)))
        .gauge_style(
            Style::default()
                .fg(usage_color((mem_pct * 100.0) as f32))
                .bg(Color::DarkGray),
        )
        .use_unicode(true);
    f.render_widget(mem_gauge, gauge_rows[0]);

    let swap_gauge = Gauge::default()
        .label(format!("Swap {:.1}%", swap_pct * 100.0))
        .ratio(swap_pct)
        .style(Style::default().fg(usage_color((swap_pct * 100.0) as f32)))
        .gauge_style(
            Style::default()
                .fg(usage_color((swap_pct * 100.0) as f32))
                .bg(Color::DarkGray),
        )
        .use_unicode(true);
    f.render_widget(swap_gauge, gauge_rows[1]);

    // History chart
    let data: Vec<(f64, f64)> = app
        .mem_history
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();

    let dataset = Dataset::default()
        .name("mem%")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Green))
        .data(&data);

    let chart = Chart::new(vec![dataset])
        .block(Block::default().borders(Borders::ALL).title(" Memory History "))
        .x_axis(
            Axis::default()
                .bounds([0.0, HISTORY_LEN as f64])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(axis_labels(vec!["0", "50", "100"]))
                .style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(chart, cols[1]);
}

// ── Disk ──────────────────────────────────────────────────────────────────────

fn draw_disk(f: &mut Frame, app: &App, area: Rect) {
    let max = app
        .disk_read_history
        .iter()
        .chain(app.disk_write_history.iter())
        .cloned()
        .fold(1.0_f64, f64::max);

    let read_data: Vec<(f64, f64)> = app
        .disk_read_history
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();
    let write_data: Vec<(f64, f64)> = app
        .disk_write_history
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name(format!("R {}", fmt_bytes(app.disk_read_bytes)))
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Yellow))
            .data(&read_data),
        Dataset::default()
            .name(format!("W {}", fmt_bytes(app.disk_write_bytes)))
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Magenta))
            .data(&write_data),
    ];

    let disk_max_label = fmt_bytes(max as u64);
    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL).title(" Disk I/O "))
        .x_axis(
            Axis::default()
                .bounds([0.0, HISTORY_LEN as f64])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max])
                .labels(axis_labels(vec!["0", &disk_max_label]))
                .style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(chart, area);
}

// ── Network ───────────────────────────────────────────────────────────────────

fn draw_network(f: &mut Frame, app: &App, area: Rect) {
    let max = app
        .net_rx_history
        .iter()
        .chain(app.net_tx_history.iter())
        .cloned()
        .fold(1.0_f64, f64::max);

    let rx_data: Vec<(f64, f64)> = app
        .net_rx_history
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();
    let tx_data: Vec<(f64, f64)> = app
        .net_tx_history
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name(format!("↓ {}", fmt_bytes(app.net_rx_bytes)))
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&rx_data),
        Dataset::default()
            .name(format!("↑ {}", fmt_bytes(app.net_tx_bytes)))
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Red))
            .data(&tx_data),
    ];

    let net_max_label = fmt_bytes(max as u64);
    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL).title(" Network I/O "))
        .x_axis(
            Axis::default()
                .bounds([0.0, HISTORY_LEN as f64])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max])
                .labels(axis_labels(vec!["0", &net_max_label]))
                .style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(chart, area);
}

// ── Processes ─────────────────────────────────────────────────────────────────

fn draw_processes(f: &mut Frame, app: &App, area: Rect) {
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let sort_indicator = |col: SortBy| {
        if app.sort_by == col { " ▼" } else { "" }
    };

    let header = Row::new(vec![
        format!("PID{}", sort_indicator(SortBy::Pid)),
        format!("NAME{}", sort_indicator(SortBy::Name)),
        format!("CPU%{}", sort_indicator(SortBy::Cpu)),
        format!("MEM (MB){}", sort_indicator(SortBy::Memory)),
    ])
    .style(header_style)
    .height(1);

    let visible_rows = area.height.saturating_sub(3) as usize; // borders(2) + header(1)
    let scroll = app.process_scroll.min(
        app.processes.len().saturating_sub(visible_rows),
    );

    let rows: Vec<Row> = app
        .processes
        .iter()
        .skip(scroll)
        .take(visible_rows)
        .map(|p| {
            let cpu_color = usage_color(p.cpu);
            Row::new(vec![
                p.pid.to_string(),
                p.name.clone(),
                format!("{:.1}", p.cpu),
                format!("{:.1}", p.mem_mb),
            ])
            .style(Style::default().fg(if p.cpu > 50.0 {
                cpu_color
            } else {
                Color::Reset
            }))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Processes ({}) ", app.processes.len())),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(table, area);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn usage_color(pct: f32) -> Color {
    if pct >= 80.0 {
        Color::Red
    } else if pct >= 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn fmt_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
