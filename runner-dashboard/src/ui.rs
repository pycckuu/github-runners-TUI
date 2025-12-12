use crate::app::{App, AppMode};
use crate::runner::RunnerStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

const BAR_WIDTH: usize = 20;
const BYTES_TO_GB: f64 = 1024.0 * 1024.0 * 1024.0;

/// Converts bytes to gigabytes.
fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / BYTES_TO_GB
}

/// Returns the color associated with a runner status.
fn status_color(status: &RunnerStatus) -> Color {
    match status {
        RunnerStatus::Active => Color::Green,
        RunnerStatus::Inactive => Color::Yellow,
        RunnerStatus::Failed => Color::Red,
        RunnerStatus::NotFound => Color::DarkGray,
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // System stats
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);

    match app.mode {
        AppMode::Help => draw_help(frame, chunks[1]),
        AppMode::Logs => draw_logs_view(frame, app, chunks[1]),
        AppMode::Normal => draw_runners_list(frame, app, chunks[1]),
    }

    draw_system_stats(frame, app, chunks[2]);
    draw_status_bar(frame, app, chunks[3]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let (active, failed, total) = app.counts();

    let title = vec![
        Span::styled(
            " Runner Dashboard ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("● {} active", active),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("✗ {} failed", failed),
            Style::default().fg(if failed > 0 {
                Color::Red
            } else {
                Color::DarkGray
            }),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("{} total", total),
            Style::default().fg(Color::White),
        ),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(Line::from(title))
        .block(block)
        .style(Style::default());

    frame.render_widget(paragraph, area);
}

fn draw_runners_list(frame: &mut Frame, app: &App, area: Rect) {
    // Split into runners list and details
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Runners list
    let items: Vec<ListItem> = app
        .runners
        .iter()
        .enumerate()
        .map(|(i, runner)| {
            let status_style = Style::default().fg(status_color(&runner.status));

            let selected = i == app.selected;
            let line_style = if selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = Line::from(vec![
                Span::styled(format!(" {} ", runner.status.symbol()), status_style),
                Span::styled(format!("{}/{}", runner.repo, runner.name), line_style),
            ]);

            ListItem::new(content).style(line_style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Runners ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_widget(list, chunks[0]);

    // Runner details
    draw_runner_details(frame, app, chunks[1]);
}

fn draw_runner_details(frame: &mut Frame, app: &App, area: Rect) {
    let details = if let Some(runner) = app.selected_runner() {
        let color = status_color(&runner.status);
        let display_name = runner.display_name();
        let status_text = format!("{} {}", runner.status.symbol(), runner.status.as_str());
        let path_str = runner.path.to_string_lossy().to_string();

        vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(display_name),
            ]),
            Line::from(vec![
                Span::styled("Repository: ", Style::default().fg(Color::Cyan)),
                Span::raw(runner.repo.clone()),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::styled(status_text, Style::default().fg(color)),
            ]),
            Line::from(vec![
                Span::styled("Service: ", Style::default().fg(Color::Cyan)),
                Span::raw(runner.service_name.clone()),
            ]),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::Cyan)),
                Span::raw(path_str),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions: ",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::raw(
                "  [s] Start  [x] Stop  [r] Restart  [l] Logs",
            )]),
        ]
    } else {
        vec![Line::from("No runner selected")]
    };

    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(details).block(block);

    frame.render_widget(paragraph, area);
}

fn draw_logs_view(frame: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(runner) = app.selected_runner() {
        format!(" Logs: {} ", runner.display_name())
    } else {
        " Logs ".to_string()
    };

    let logs: Vec<Line> = app
        .logs
        .iter()
        .skip(app.log_scroll)
        .map(|log| {
            let log_lower = log.to_lowercase();
            let style = if log_lower.contains("error") {
                Style::default().fg(Color::Red)
            } else if log_lower.contains("warn") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            Line::styled(log.as_str(), style)
        })
        .collect();

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let paragraph = Paragraph::new(logs).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ↑/k      Move up"),
        Line::from("  ↓/j      Move down"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  s        Start selected runner"),
        Line::from("  x        Stop selected runner"),
        Line::from("  r        Restart selected runner"),
        Line::from("  l        Toggle logs view"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "General",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ?/h      Toggle this help"),
        Line::from("  q        Quit"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "In Logs View",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ↑/k      Scroll up"),
        Line::from("  ↓/j      Scroll down"),
        Line::from("  l/Esc    Exit logs view"),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(help_text).block(block);

    frame.render_widget(paragraph, area);
}

fn draw_system_stats(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.system_stats;

    let cpu_bar = create_bar(stats.cpu_usage as f64, 100.0, BAR_WIDTH);
    let mem_percent = if stats.memory_total > 0 {
        (stats.memory_used as f64 / stats.memory_total as f64) * 100.0
    } else {
        0.0
    };
    let mem_bar = create_bar(mem_percent, 100.0, BAR_WIDTH);

    let mem_used_gb = bytes_to_gb(stats.memory_used);
    let mem_total_gb = bytes_to_gb(stats.memory_total);

    let content = Line::from(vec![
        Span::styled(" CPU: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            cpu_bar,
            Style::default().fg(cpu_color(stats.cpu_usage as f64)),
        ),
        Span::raw(format!(" {:5.1}%", stats.cpu_usage)),
        Span::raw("  |  "),
        Span::styled("MEM: ", Style::default().fg(Color::Cyan)),
        Span::styled(mem_bar, Style::default().fg(mem_color(mem_percent))),
        Span::raw(format!(" {:.1}/{:.1} GB", mem_used_gb, mem_total_gb)),
        Span::raw("  |  "),
        Span::styled("Load: ", Style::default().fg(Color::Cyan)),
        Span::raw(format!(
            "{:.2} {:.2} {:.2}",
            stats.load_avg[0], stats.load_avg[1], stats.load_avg[2]
        )),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(content).block(block);

    frame.render_widget(paragraph, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let message = app.status_message.as_deref().unwrap_or("");

    let mode_text = match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Logs => "LOGS",
        AppMode::Help => "HELP",
    };

    let content = Line::from(vec![
        Span::styled(
            format!(" {} ", mode_text),
            Style::default().bg(Color::Blue).fg(Color::White),
        ),
        Span::raw(" "),
        Span::raw(message),
        Span::raw("  "),
        Span::styled(" ?:help q:quit ", Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, area);
}

fn create_bar(value: f64, max: f64, width: usize) -> String {
    let filled = ((value / max) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Returns a color based on percentage usage and thresholds.
/// Red if above high threshold, yellow if above medium threshold, otherwise green.
fn usage_color(percent: f64, medium_threshold: f64, high_threshold: f64) -> Color {
    if percent > high_threshold {
        Color::Red
    } else if percent > medium_threshold {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn cpu_color(percent: f64) -> Color {
    usage_color(percent, 50.0, 80.0)
}

fn mem_color(percent: f64) -> Color {
    usage_color(percent, 70.0, 90.0)
}
