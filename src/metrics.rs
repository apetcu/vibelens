use std::collections::HashMap;

use crate::models::{DataSource, GlobalMetrics, ParsedSession, ProjectSummary, TimelineEntry, TokenTotals};
use crate::format::estimate_cost;

pub fn build_project_summaries(
    projects: Vec<(String, String, Vec<ParsedSession>, Vec<DataSource>)>,
) -> Vec<ProjectSummary> {
    let mut summaries: Vec<ProjectSummary> = Vec::new();

    for (project_id, project_dir, sessions, sources) in projects {
        if sessions.is_empty() {
            continue;
        }

        // Derive project name: session cwd > project dir > project_id
        let path = sessions
            .iter()
            .find(|s| !s.cwd.is_empty())
            .map(|s| s.cwd.clone())
            .or_else(|| {
                if !project_dir.is_empty() {
                    Some(project_dir.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let name = if !path.is_empty() {
            path.split('/').last().unwrap_or(&project_id).to_string()
        } else {
            project_id.clone()
        };

        let mut tokens = TokenTotals::zero();
        let mut tool_usage: HashMap<String, u64> = HashMap::new();
        let mut message_count = 0usize;
        let mut lines_added = 0u64;
        let mut lines_removed = 0u64;
        let mut last_active = String::new();
        let mut model = String::new();

        for s in &sessions {
            tokens.input += s.total_tokens.input;
            tokens.output += s.total_tokens.output;
            tokens.cache_read += s.total_tokens.cache_read;
            tokens.cache_creation += s.total_tokens.cache_creation;
            message_count += s.messages.len();
            lines_added += s.lines_added;
            lines_removed += s.lines_removed;

            for (tool, count) in &s.tool_usage {
                *tool_usage.entry(tool.clone()).or_insert(0) += count;
            }

            if !s.last_active.is_empty() && s.last_active > last_active {
                last_active = s.last_active.clone();
            }
            if model.is_empty() && !s.model.is_empty() {
                model = s.model.clone();
            }
        }

        let cost = estimate_cost(
            &model,
            tokens.input,
            tokens.output,
            tokens.cache_read,
        );

        let session_count = sessions.len();
        summaries.push(ProjectSummary {
            id: project_id,
            name,
            path,
            session_count,
            message_count,
            total_tokens: tokens,
            lines_added,
            lines_removed,
            last_active,
            tool_usage,
            cost,
            model,
            sessions,
            sources,
        });
    }

    // Sort by last_active descending
    summaries.sort_by(|a, b| b.last_active.cmp(&a.last_active));
    summaries
}

pub fn compute_global_metrics(projects: &[ProjectSummary]) -> GlobalMetrics {
    let mut tokens = TokenTotals::zero();
    let mut tool_usage: HashMap<String, u64> = HashMap::new();
    let mut total_messages = 0usize;
    let mut total_sessions = 0usize;
    let mut lines_added = 0u64;
    let mut lines_removed = 0u64;
    let mut total_cost = 0.0f64;
    let mut human_lines = 0u64;
    let mut human_words = 0u64;
    let mut human_chars = 0u64;

    let mut day_map: HashMap<String, TimelineEntry> = HashMap::new();

    for p in projects {
        tokens.input += p.total_tokens.input;
        tokens.output += p.total_tokens.output;
        tokens.cache_read += p.total_tokens.cache_read;
        tokens.cache_creation += p.total_tokens.cache_creation;
        total_messages += p.message_count;
        total_sessions += p.session_count;
        lines_added += p.lines_added;
        lines_removed += p.lines_removed;
        total_cost += p.cost;

        for (tool, count) in &p.tool_usage {
            *tool_usage.entry(tool.clone()).or_insert(0) += count;
        }

        for s in &p.sessions {
            human_lines += s.human_lines;
            human_words += s.human_words;
            human_chars += s.human_chars;

            if !s.started_at.is_empty() {
                let day = s.started_at.split('T').next().unwrap_or("").to_string();
                if !day.is_empty() {
                    let entry = day_map.entry(day.clone()).or_insert(TimelineEntry {
                        date: day,
                        sessions: 0,
                        messages: 0,
                        token_input: 0,
                        token_output: 0,
                        claude_sessions: 0,
                        cursor_sessions: 0,
                    });
                    entry.sessions += 1;
                    entry.messages += s.messages.len() as u64;
                    entry.token_input += s.total_tokens.input;
                    entry.token_output += s.total_tokens.output;
                    match s.source {
                        DataSource::Claude => entry.claude_sessions += 1,
                        DataSource::Cursor => entry.cursor_sessions += 1,
                    }
                }
            }
        }
    }

    let mut timeline: Vec<TimelineEntry> = day_map.into_values().collect();
    timeline.sort_by(|a, b| a.date.cmp(&b.date));

    GlobalMetrics {
        total_projects: projects.len(),
        total_sessions,
        total_messages,
        total_tokens: tokens,
        tool_usage,
        timeline,
        total_lines_added: lines_added,
        total_lines_removed: lines_removed,
        total_cost,
        human_lines,
        human_words,
        human_chars,
    }
}
