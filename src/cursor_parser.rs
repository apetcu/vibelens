use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::models::{
    ConversationMessage, DataSource, FileContribution, ParsedSession, TokenTotals,
};

/// Cursor tool names → normalized names (matching Claude Code conventions).
fn normalize_tool(name: &str) -> &str {
    match name {
        "edit_file" => "Edit",
        "create_file" => "Write",
        "run_terminal_command" => "Bash",
        "read_file" => "Read",
        "list_directory" => "Glob",
        "file_search" => "Glob",
        "search_files" => "Grep",
        "codebase_search" => "Grep",
        "grep_search" => "Grep",
        other => other,
    }
}

/// Bubble type constants
const BUBBLE_USER: i64 = 1;
const BUBBLE_ASSISTANT: i64 = 2;

fn cursor_global_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Library")
        .join("Application Support")
        .join("Cursor")
        .join("User")
        .join("globalStorage")
        .join("state.vscdb")
}

#[derive(Debug, serde::Deserialize)]
struct CursorBubble {
    #[serde(rename = "type")]
    bubble_type: Option<i64>,
    text: Option<String>,
    #[serde(rename = "tokenCount")]
    token_count: Option<BubbleTokenCount>,
    #[serde(rename = "codeBlocks")]
    code_blocks: Option<Vec<CursorCodeBlock>>,
    #[serde(rename = "timingInfo")]
    timing_info: Option<BubbleTiming>,
    #[serde(rename = "bubbleId")]
    bubble_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct BubbleTokenCount {
    #[serde(rename = "inputTokens")]
    input_tokens: Option<u64>,
    #[serde(rename = "outputTokens")]
    output_tokens: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct CursorCodeBlock {
    content: Option<String>,
    uri: Option<CodeBlockUri>,
}

#[derive(Debug, serde::Deserialize)]
struct CodeBlockUri {
    #[serde(rename = "_fsPath")]
    fs_path: Option<String>,
    path: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct BubbleTiming {
    #[serde(rename = "clientStartTime")]
    client_start_time: Option<f64>,
    #[serde(rename = "clientEndTime")]
    client_end_time: Option<f64>,
    #[serde(rename = "clientSettleTime")]
    client_settle_time: Option<f64>,
}

#[derive(Debug, serde::Deserialize)]
struct ComposerHead {
    #[serde(rename = "composerId")]
    composer_id: String,
    #[serde(rename = "createdAt")]
    created_at: Option<f64>,
}

#[derive(Debug, serde::Deserialize)]
struct ComposerData {
    #[serde(rename = "allComposers")]
    all_composers: Option<Vec<ComposerHead>>,
}

/// Parse a Cursor session (composer) from SQLite databases.
pub fn parse_cursor_session(
    db_path: &str,
    session_id: &str,
    project_id: &str,
) -> Result<ParsedSession> {
    let bubbles = load_bubbles_from_global(session_id)?;
    let created_at = get_composer_created_at(db_path, session_id);

    Ok(build_parsed_session(
        &bubbles,
        session_id,
        project_id,
        &created_at,
    ))
}

/// Look up composer createdAt from the workspace state.vscdb.
fn get_composer_created_at(db_path: &str, composer_id: &str) -> String {
    let conn = match Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    for table in &["ItemTable", "cursorDiskKV"] {
        let sql = format!(
            "SELECT value FROM {} WHERE key = 'composer.composerData'",
            table
        );
        let row: Option<String> = conn.query_row(&sql, [], |row| row.get(0)).ok();

        if let Some(value) = row {
            if let Ok(data) = serde_json::from_str::<ComposerData>(&value) {
                if let Some(composers) = data.all_composers {
                    if let Some(composer) = composers.iter().find(|c| c.composer_id == composer_id)
                    {
                        if let Some(created_at) = composer.created_at {
                            if created_at > 1_000_000_000.0 {
                                // May be in ms or seconds
                                let ts_ms = if created_at > 1_000_000_000_000.0 {
                                    created_at as i64
                                } else {
                                    (created_at * 1000.0) as i64
                                };
                                if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts_ms) {
                                    return dt.to_rfc3339();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    String::new()
}

/// Load bubbles from the global Cursor state.vscdb.
fn load_bubbles_from_global(composer_id: &str) -> Result<Vec<CursorBubble>> {
    let db_path = cursor_global_db_path();
    if !db_path.exists() {
        return Ok(vec![]);
    }

    let conn = Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

    let pattern = format!("bubbleId:{}:%", composer_id);
    let mut stmt = conn.prepare("SELECT value FROM cursorDiskKV WHERE key LIKE ?1")?;

    let mut bubbles: Vec<CursorBubble> = stmt
        .query_map([&pattern], |row| {
            let value: String = row.get(0)?;
            Ok(value)
        })?
        .filter_map(|r| r.ok())
        .filter_map(|value| serde_json::from_str(&value).ok())
        .collect();

    // Sort by timing
    bubbles.sort_by(|a, b| {
        let ta = a
            .timing_info
            .as_ref()
            .and_then(|t| t.client_start_time)
            .unwrap_or(0.0);
        let tb = b
            .timing_info
            .as_ref()
            .and_then(|t| t.client_start_time)
            .unwrap_or(0.0);
        ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(bubbles)
}

fn resolve_timestamp(raw: Option<f64>, base_epoch_ms: f64) -> Option<f64> {
    let val = raw?;
    if val <= 0.0 {
        return None;
    }
    if val > 1_000_000_000_000.0 {
        // Absolute epoch ms
        Some(val)
    } else if base_epoch_ms > 0.0 {
        // Relative offset
        Some(base_epoch_ms + val)
    } else {
        None
    }
}

fn ms_to_iso(ms: f64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default()
}

fn build_parsed_session(
    bubbles: &[CursorBubble],
    session_id: &str,
    project_id: &str,
    composer_created_at: &str,
) -> ParsedSession {
    let mut messages: Vec<ConversationMessage> = Vec::new();
    let mut tool_usage: HashMap<String, u64> = HashMap::new();
    let mut tokens = TokenTotals::zero();
    let mut lines_added: u64 = 0;
    let lines_removed: u64 = 0;
    let mut file_contributions: HashMap<String, FileContribution> = HashMap::new();
    let mut first_prompt = String::new();
    let mut started_at = String::new();
    let mut last_active = String::new();
    let mut human_lines: u64 = 0;
    let mut human_words: u64 = 0;
    let mut human_chars: u64 = 0;
    let mut duration_ms: f64 = 0.0;

    // Base epoch ms from composer createdAt
    let base_epoch_ms: f64 = if !composer_created_at.is_empty() {
        chrono::DateTime::parse_from_rfc3339(composer_created_at)
            .map(|d| d.timestamp_millis() as f64)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    for bubble in bubbles {
        let raw_start = bubble
            .timing_info
            .as_ref()
            .and_then(|t| t.client_start_time);
        let raw_end = bubble
            .timing_info
            .as_ref()
            .and_then(|t| t.client_end_time.or(t.client_settle_time));

        let start_time = resolve_timestamp(raw_start, base_epoch_ms);
        let end_time = resolve_timestamp(raw_end, base_epoch_ms);

        let ts = start_time
            .map(|t| ms_to_iso(t))
            .unwrap_or_else(|| composer_created_at.to_string());

        if !ts.is_empty() {
            if started_at.is_empty() {
                started_at = ts.clone();
            }
            last_active = ts.clone();
        }

        let text = bubble.text.as_deref().unwrap_or("");
        let bubble_type = bubble.bubble_type.unwrap_or(0);

        if bubble_type == BUBBLE_USER {
            if first_prompt.is_empty() && !text.trim().is_empty() {
                first_prompt = text.trim().to_string();
            }

            // Count human contribution
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                human_lines += trimmed.lines().count() as u64;
                human_words += trimmed.split_whitespace().count() as u64;
                human_chars += trimmed.len() as u64;
            }

            // Token count (input tokens up to this point — Cursor reports cumulative)
            if let Some(ref tc) = bubble.token_count {
                if let Some(input) = tc.input_tokens {
                    tokens.input = tokens.input.max(input);
                }
            }

            messages.push(ConversationMessage {
                role: "user".to_string(),
                timestamp: ts,
                uuid: bubble
                    .bubble_id
                    .clone()
                    .unwrap_or_else(|| format!("cursor-{}-{}", session_id, messages.len())),
                usage: None,
                content: text.to_string(),
            });
        } else if bubble_type == BUBBLE_ASSISTANT {
            // Count tokens
            if let Some(ref tc) = bubble.token_count {
                if let Some(output) = tc.output_tokens {
                    tokens.output += output;
                }
                if let Some(input) = tc.input_tokens {
                    tokens.input = tokens.input.max(input);
                }
            }

            // Duration from timing info
            if let (Some(start), Some(end)) = (start_time, end_time) {
                duration_ms += end - start;
            }

            // Process code blocks for line counting and file contributions
            if let Some(ref code_blocks) = bubble.code_blocks {
                for cb in code_blocks {
                    if let Some(ref content) = cb.content {
                        let lines = content.lines().count() as u64;
                        lines_added += lines;

                        let fp = cb
                            .uri
                            .as_ref()
                            .and_then(|u| u.fs_path.as_deref().or(u.path.as_deref()))
                            .unwrap_or("");

                        if !fp.is_empty() {
                            let fc = file_contributions
                                .entry(fp.to_string())
                                .or_insert(FileContribution {
                                    added: 0,
                                    removed: 0,
                                });
                            fc.added += lines;

                            *tool_usage.entry(normalize_tool("edit_file").to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }

            // Truncate assistant content
            let mut content = text.to_string();
            if content.len() > 5000 {
                content.truncate(5000);
                content.push_str("...");
            }

            messages.push(ConversationMessage {
                role: "assistant".to_string(),
                timestamp: ts,
                uuid: bubble
                    .bubble_id
                    .clone()
                    .unwrap_or_else(|| format!("cursor-{}-{}", session_id, messages.len())),
                usage: None,
                content,
            });
        }
    }

    ParsedSession {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        cwd: String::new(),
        messages,
        tool_usage,
        total_tokens: tokens,
        duration_ms,
        lines_added,
        lines_removed,
        file_contributions,
        first_prompt,
        started_at: if started_at.is_empty() {
            composer_created_at.to_string()
        } else {
            started_at
        },
        last_active: if last_active.is_empty() {
            composer_created_at.to_string()
        } else {
            last_active
        },
        human_lines,
        human_words,
        human_chars,
        model: String::new(),
        source: DataSource::Cursor,
    }
}
