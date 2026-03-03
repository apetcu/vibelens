use colored::Colorize;
use tabled::{builder::Builder, settings::Style};

use crate::format::{format_cost, format_number, format_relative, short_model};
use crate::models::{DataSource, GlobalMetrics, ProjectSummary};

pub fn print_cli_table(projects: &[ProjectSummary], metrics: &GlobalMetrics) {
    // Header stats
    println!();
    println!(
        "{}  {} projects  {} sessions  {} messages  {} tokens  {} cost",
        "Claude Tracker".bold().cyan(),
        metrics.total_projects.to_string().bold(),
        metrics.total_sessions.to_string().bold(),
        format_number(metrics.total_messages as u64).bold(),
        format_number(metrics.total_tokens.total()).bold(),
        format_cost(metrics.total_cost).bold().green(),
    );
    println!(
        "  Lines: {} added / {} removed",
        format_number(metrics.total_lines_added).green(),
        format_number(metrics.total_lines_removed).red(),
    );
    println!();

    // Project table
    let mut builder = Builder::default();
    builder.push_record([
        "Project",
        "Source",
        "Sessions",
        "Messages",
        "Tokens",
        "Lines +/-",
        "Cost",
        "Model",
        "Last Active",
    ]);

    for p in projects {
        let source_label = source_label_str(&p.sources);
        builder.push_record([
            &p.name,
            &source_label,
            &p.session_count.to_string(),
            &p.message_count.to_string(),
            &format_number(p.total_tokens.total()),
            &format!("{}/{}", format_number(p.lines_added), format_number(p.lines_removed)),
            &format_cost(p.cost),
            &short_model(&p.model),
            &format_relative(&p.last_active),
        ]);
    }

    let table = builder.build().with(Style::rounded()).to_string();
    println!("{}", table);
    println!();
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

pub fn print_json(projects: &[ProjectSummary], metrics: &GlobalMetrics) {
    #[derive(serde::Serialize)]
    struct Output<'a> {
        metrics: &'a GlobalMetrics,
        projects: Vec<ProjectJson<'a>>,
    }

    #[derive(serde::Serialize)]
    struct ProjectJson<'a> {
        name: &'a str,
        path: &'a str,
        source: String,
        session_count: usize,
        message_count: usize,
        tokens_total: u64,
        lines_added: u64,
        lines_removed: u64,
        cost: f64,
        model: &'a str,
        last_active: &'a str,
    }

    let output = Output {
        metrics,
        projects: projects
            .iter()
            .map(|p| ProjectJson {
                name: &p.name,
                path: &p.path,
                source: source_label_str(&p.sources),
                session_count: p.session_count,
                message_count: p.message_count,
                tokens_total: p.total_tokens.total(),
                lines_added: p.lines_added,
                lines_removed: p.lines_removed,
                cost: p.cost,
                model: &p.model,
                last_active: &p.last_active,
            })
            .collect(),
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
