use std::{collections::HashMap, io, time::Duration};

use anyhow::Result;
use ratatui::{
    Frame, Terminal,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use serde::Deserialize;
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
};

use crate::{
    exec::metrics::{ExecMetrics, JobMetrics},
    logging::Logger,
};

#[derive(Debug, Deserialize)]
pub struct ProjectMetrics {
    pub project_id: String,
    pub project_name: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u128,
    pub cpu_usage: f32,
    pub mem_usage_kb: u64,
    pub mem_usage: f32,
    pub max_cpu: f32,
    pub max_mem: f32,
    pub jobs: HashMap<String, JobMetrics>,
}
#[derive(Debug, Deserialize, PartialEq)]
pub struct ProjectStats {
    pub id: String,
    pub name: String,
    pub last_duration: String,
    pub avg_cpu: f32,
    pub avg_mem: f32,
    pub max_cpu: f32,
    pub max_mem: f32,
    pub mem_kb: u64,
    pub runs: usize,
    pub last_logs: Vec<String>,
}

pub struct App {
    pub project: Vec<ProjectStats>,
    pub selected: usize,
    pub scroll: usize,
    pub table_height: usize,
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)].as_ref())
        .split(f.area());

    let table_height = chunks[0].height.saturating_sub(3) as usize;
    app.table_height = table_height;
    let table = render_table(app, table_height);
    f.render_widget(table, chunks[0]);

    if let Some(details) = render_project_details(app) {
        f.render_widget(details, chunks[1]);
    }
}

pub fn render_project_details(app: &App) -> Option<Paragraph<'_>> {
    if let Some(proj) = app.project.get(app.selected) {
        let logs = proj
            .last_logs
            .iter()
            .map(|l| format!("  {}", l))
            .collect::<Vec<_>>()
            .join("\n");

        let text = format!(
            "Last run: {}\nAvg CPU: {:.1}%\nAvg MEM: {:.1}%\nTotal runs: {}\nLogs:\n{}",
            proj.last_duration, proj.avg_cpu, proj.avg_mem, proj.runs, logs
        );

        let paragraph =
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Details"));

        return Some(paragraph);
    }
    None
}

pub fn render_table(app: &App, height: usize) -> Table<'_> {
    // En-tête
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Name"),
        Cell::from("Last Duration"),
        Cell::from("CPU %"),
        Cell::from("MEM %"),
        Cell::from("Runs"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .project
        .iter()
        .enumerate()
        .skip(app.scroll)
        .take(height)
        .map(|(i, proj)| {
            let mut row = Row::new(vec![
                proj.id.clone(),
                proj.name.clone(),
                proj.last_duration.clone(),
                format!("{:.1}", proj.avg_cpu),
                format!("{:.1}", proj.avg_mem),
                proj.runs.to_string(),
            ]);

            if i == app.selected {
                row = row.style(Style::default().bg(Color::Blue).fg(Color::White));
            }
            row
        })
        .collect();

    Table::new(
        rows,
        &[
            Constraint::Length(13),
            Constraint::Length(18),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(6),
        ],
    )
    .header(header)
    .block(Block::default().title("Projects").borders(Borders::ALL))
}

pub async fn load_all_stats() -> Result<Vec<ProjectStats>> {
    let dir = ExecMetrics::ensure_metrics_dir().await?;
    let mut entries = fs::read_dir(&dir).await?;
    let mut projects: HashMap<String, Vec<ProjectMetrics>> = HashMap::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ndjson") {
            continue;
        }

        let file = fs::File::open(&path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ProjectMetrics>(&line) {
                Ok(pm) => {
                    projects.entry(pm.project_id.clone()).or_default().push(pm);
                }
                Err(e) => {
                    eprintln!("JSON Error in {:?}: {e}", path);
                }
            }
        }
    }

    let mut stats = Vec::new();
    for (_id, runs) in projects {
        if runs.is_empty() {
            continue;
        }

        let id = runs[0].project_id.clone();
        let name = runs[0].project_name.clone();

        let runs_count = runs.len();
        let avg_cpu = runs.iter().map(|r| r.cpu_usage).sum::<f32>() / runs_count as f32;
        let avg_mem = runs.iter().map(|r| r.mem_usage).sum::<f32>() / runs_count as f32;

        let last = runs.iter().max_by_key(|r| r.finished_at).unwrap();
        let last_duration = format!("{} ms", last.duration_ms);
        let last_logs = Logger::fetchn(&id, 5)
            .await
            .unwrap_or_else(|e| vec![format!("Error: {e}")]);
        let avg_mem_kb = runs.iter().map(|r| r.mem_usage_kb).sum::<u64>() / runs_count as u64;

        stats.push(ProjectStats {
            id,
            name,
            last_duration,
            avg_cpu,
            avg_mem,
            runs: runs_count,
            last_logs,
            max_cpu: last.max_cpu,
            max_mem: last.max_mem,
            mem_kb: avg_mem_kb,
        });
    }
    // dbg!(&stats);
    stats.sort_by(|a, b| {
        let a_last = a
            .last_duration
            .replace(" ms", "")
            .parse::<u128>()
            .unwrap_or(0);
        let b_last = b
            .last_duration
            .replace(" ms", "")
            .parse::<u128>()
            .unwrap_or(0);
        b_last.cmp(&a_last) // du plus récent au plus ancien
    });
    Ok(stats)
}

pub async fn display_stats_interface() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        project: load_all_stats().await?,
        selected: 0,
        scroll: 0,
        table_height: 0,
    };

    loop {
        app.project = load_all_stats().await?;

        terminal.draw(|f| {
            ui(f, &mut app);
        })?;

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => break,
                KeyCode::Down => {
                    if !app.project.is_empty() {
                        app.selected = (app.selected + 1).min(app.project.len() - 1);
                        if app.selected >= app.scroll + app.table_height {
                            app.scroll = app.selected - app.table_height + 1;
                        }
                    }
                }
                KeyCode::Up => {
                    if !app.project.is_empty() && app.selected > 0 {
                        app.selected -= 1;

                        if app.selected < app.scroll {
                            app.scroll = app.selected;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
