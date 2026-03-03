mod cursor_parser;
mod cursor_scanner;
mod display;
mod format;
mod metrics;
mod models;
mod parser;
mod scanner;
mod theme;
mod tui_app;
mod tui_events;
mod tui_ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use rayon::prelude::*;
use std::io;
use std::sync::mpsc;

use crate::display::{print_cli_table, print_json};
use crate::metrics::{build_project_summaries, compute_global_metrics};
use crate::models::{DataSource, ParsedSession};
use crate::scanner::{scan_all_projects, scan_claude_projects};
use crate::tui_app::App;

#[derive(Parser)]
#[command(name = "ct", about = "Claude Tracker — analyze Claude Code & Cursor usage")]
struct Cli {
    /// Print table output instead of interactive TUI
    #[arg(long)]
    cli: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

use crate::tui_app::LoadMessage;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Load all data (scan + parse + aggregate), optionally sending progress
fn load_data(
    progress: Option<mpsc::Sender<LoadMessage>>,
) -> Result<(Vec<crate::models::ProjectSummary>, crate::models::GlobalMetrics)> {
    let send = |msg: &str| {
        if let Some(ref tx) = progress {
            let _ = tx.send(LoadMessage::Progress(msg.to_string()));
        }
    };

    send("Scanning Claude projects...");
    let claude_projects = scan_claude_projects()?;

    send("Scanning Cursor workspaces...");
    let cursor_projects = cursor_scanner::scan_cursor_projects().unwrap_or_default();

    send("Merging projects...");
    let scanned = scan_all_projects(claude_projects, cursor_projects);
    let total = scanned.len();

    let counter = Arc::new(AtomicUsize::new(0));
    let progress_tx = progress.clone();

    let project_sessions: Vec<(String, String, Vec<ParsedSession>, Vec<DataSource>)> = scanned
        .into_par_iter()
        .map(|project| {
            let project_id = project.id.clone();
            let sources = project.sources.clone();
            let dir = project.dir.clone();

            // Report progress
            let n = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let name = dir.split('/').last().unwrap_or(&project_id);
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(LoadMessage::Progress(format!(
                    "Parsing: {} ({}/{})",
                    name, n, total
                )));
            }

            let sessions: Vec<ParsedSession> = project
                .session_files
                .par_iter()
                .filter_map(|sf| match sf.source {
                    DataSource::Claude => {
                        parser::parse_session_file(&sf.path, &sf.id, &project_id).ok()
                    }
                    DataSource::Cursor => {
                        cursor_parser::parse_cursor_session(&sf.path, &sf.id, &project_id).ok()
                    }
                })
                .collect();
            (project_id, dir, sessions, sources)
        })
        .collect();

    send("Building metrics...");
    let projects = build_project_summaries(project_sessions);
    let metrics = compute_global_metrics(&projects);
    Ok((projects, metrics))
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // Non-TUI modes: load synchronously
    if args.json || args.cli {
        let (projects, metrics) = load_data(None)?;
        if args.json {
            print_json(&projects, &metrics);
        } else {
            print_cli_table(&projects, &metrics);
        }
        return Ok(());
    }

    // TUI mode: show immediately, load in background
    run_tui()
}

fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Spawn background data loading with progress
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let progress_tx = tx.clone();
        if let Ok((projects, metrics)) = load_data(Some(progress_tx)) {
            let _ = tx.send(LoadMessage::Done(projects, metrics));
        }
    });

    let mut app = App::loading(rx);

    // Main loop
    loop {
        // Check for loaded data
        app.poll_load();

        terminal.draw(|f| tui_ui::draw(f, &mut app))?;
        tui_events::handle_events(&mut app)?;

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
