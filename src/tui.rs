use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, TableState};
use ratatui::Terminal;
use std::io;
use std::time::{Duration, Instant};

use crate::history;
use crate::providers::{DeployStatus, Status};

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

struct App {
    entries: Vec<DeployStatus>,
    table_state: TableState,
    should_quit: bool,
    last_refresh: Instant,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            entries: vec![],
            table_state: TableState::default(),
            should_quit: false,
            last_refresh: Instant::now(),
        };
        app.refresh();
        if !app.entries.is_empty() {
            app.table_state.select(Some(0));
        }
        app
    }

    fn refresh(&mut self) {
        let filter = history::HistoryFilter {
            limit: 100,
            repo: None,
        };
        self.entries = history::read(&filter).unwrap_or_default();
        self.last_refresh = Instant::now();
    }

    fn selected_entry(&self) -> Option<&DeployStatus> {
        self.table_state.selected().and_then(|i| self.entries.get(i))
    }

    fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        let next = if i >= self.entries.len() - 1 { 0 } else { i + 1 };
        self.table_state.select(Some(next));
    }

    fn prev(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        let prev = if i == 0 { self.entries.len() - 1 } else { i - 1 };
        self.table_state.select(Some(prev));
    }

    fn open_in_browser(&self) {
        if let Some(entry) = self.selected_entry() {
            if let Some(url) = &entry.url {
                let _ = std::process::Command::new("xdg-open")
                    .arg(url)
                    .spawn();
            }
        }
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        // Auto-refresh
        if app.last_refresh.elapsed() > REFRESH_INTERVAL {
            app.refresh();
        }

        // Handle input (non-blocking, 200ms timeout)
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.prev(),
                    KeyCode::Char('o') | KeyCode::Enter => app.open_in_browser(),
                    KeyCode::Char('r') => app.refresh(),
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn draw(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // Header
        Constraint::Min(10),   // Table
        Constraint::Length(6), // Details
        Constraint::Length(1), // Status bar
    ])
    .split(f.area());

    draw_header(f, chunks[0]);
    draw_table(f, app, chunks[1]);
    draw_details(f, app, chunks[2]);
    draw_statusbar(f, app, chunks[3]);
}

fn draw_header(f: &mut ratatui::Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("  B E A C O N", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled("  —  Deploy Dashboard", Style::default().fg(Color::DarkGray)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Rgb(49, 50, 68)))
            .padding(Padding::new(0, 0, 1, 0)),
    );
    f.render_widget(header, area);
}

fn draw_table(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("  "),
        Cell::from("Repo").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Workflow").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Branch").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Commit").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Time").style(Style::default().fg(Color::DarkGray)),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .entries
        .iter()
        .map(|entry| {
            let (icon, color) = match entry.status {
                Status::Success => ("✓", Color::Green),
                Status::Failed => ("✗", Color::Red),
                Status::InProgress => ("◉", Color::Yellow),
                Status::NotFound => ("?", Color::DarkGray),
            };

            let short_repo = entry.repo.split('/').last().unwrap_or(&entry.repo);
            let commit = if entry.commit.len() > 7 {
                &entry.commit[..7]
            } else {
                &entry.commit
            };
            let workflow = entry.workflow_name.as_deref().unwrap_or("-");
            let ago = time_ago(&entry.timestamp);

            Row::new(vec![
                Cell::from(format!(" {icon}")).style(Style::default().fg(color)),
                Cell::from(short_repo.to_string()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Cell::from(workflow.to_string()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(entry.branch.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(commit.to_string()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(ago).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(3),
        Constraint::Min(15),
        Constraint::Min(18),
        Constraint::Min(10),
        Constraint::Length(8),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .padding(Padding::new(1, 1, 0, 0)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 30, 46))
                .add_modifier(Modifier::BOLD),
        )
        .row_highlight_style(Style::default().bg(Color::Rgb(30, 30, 46)));

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_details(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let content = if let Some(entry) = app.selected_entry() {
        let (status_text, color) = match entry.status {
            Status::Success => ("SUCCESS", Color::Green),
            Status::Failed => ("FAILED", Color::Red),
            Status::InProgress => ("IN PROGRESS", Color::Yellow),
            Status::NotFound => ("NOT FOUND", Color::DarkGray),
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(format!(" {status_text}"), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {}  ", entry.repo), Style::default().fg(Color::White)),
                Span::styled(
                    entry.url.as_deref().unwrap_or(""),
                    Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
                ),
            ]),
        ];

        if !entry.failed_jobs.is_empty() {
            let jobs_text = entry.failed_jobs.join(", ");
            lines.push(Line::from(vec![
                Span::styled(" Failed: ", Style::default().fg(Color::Red)),
                Span::styled(jobs_text, Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines
    } else {
        vec![Line::from(Span::styled(
            " No entry selected",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let details = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Rgb(49, 50, 68)))
            .title(" Details ")
            .title_style(Style::default().fg(Color::DarkGray))
            .padding(Padding::new(0, 0, 1, 0)),
    );
    f.render_widget(details, area);
}

fn draw_statusbar(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let count = app.entries.len();
    let success = app.entries.iter().filter(|e| e.status == Status::Success).count();
    let failed = app.entries.iter().filter(|e| e.status == Status::Failed).count();

    let bar = Line::from(vec![
        Span::styled(" ↑↓", Style::default().fg(Color::DarkGray)),
        Span::styled(" navigate  ", Style::default().fg(Color::Rgb(88, 91, 112))),
        Span::styled("o", Style::default().fg(Color::DarkGray)),
        Span::styled(" open  ", Style::default().fg(Color::Rgb(88, 91, 112))),
        Span::styled("r", Style::default().fg(Color::DarkGray)),
        Span::styled(" refresh  ", Style::default().fg(Color::Rgb(88, 91, 112))),
        Span::styled("q", Style::default().fg(Color::DarkGray)),
        Span::styled(" quit", Style::default().fg(Color::Rgb(88, 91, 112))),
        Span::styled(
            format!("   {count} deploys  "),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(format!("✓{success}"), Style::default().fg(Color::Green)),
        Span::styled("  ", Style::default()),
        Span::styled(format!("✗{failed}"), Style::default().fg(Color::Red)),
    ]);

    f.render_widget(Paragraph::new(bar), area);
}

fn time_ago(ts: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(*ts);
    let secs = diff.num_seconds();

    if secs < 0 {
        "now".to_string()
    } else if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}
