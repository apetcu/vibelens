use anyhow::Result;
use std::collections::HashMap;
use std::fs;

use crate::models::{
    ConversationMessage, DataSource, FileContribution, ParsedSession, RawEvent, TokenTotals,
};

const SKIP_TYPES: &[&str] = &["progress", "queue-operation", "file-history-snapshot"];

struct TaggedEvent {
    kind: &'static str, // "user" or "assistant"
    event: RawEvent,
    ts: String,
}

pub fn parse_session_file(
    file_path: &str,
    session_id: &str,
    project_id: &str,
) -> Result<ParsedSession> {
    let raw = fs::read_to_string(file_path)?;

    let mut user_events: Vec<(RawEvent, String)> = Vec::new();
    let mut assistant_by_id: HashMap<String, (RawEvent, String)> = HashMap::new();
    let mut assistant_no_id: Vec<(RawEvent, String)> = Vec::new();
    let mut duration_ms: f64 = 0.0;
    let mut cwd = String::new();
    let mut started_at = String::new();
    let mut last_active = String::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let event: RawEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if SKIP_TYPES.contains(&event.event_type.as_str()) {
            continue;
        }

        if cwd.is_empty() {
            if let Some(ref c) = event.cwd {
                cwd = c.clone();
            }
        }

        let ts = event.timestamp.clone().unwrap_or_default();
        if !ts.is_empty() {
            if started_at.is_empty() {
                started_at = ts.clone();
            }
            last_active = ts.clone();
        }

        if event.event_type == "system" {
            if event.subtype.as_deref() == Some("turn_duration") {
                if let Some(d) = event.duration_ms {
                    duration_ms += d;
                }
            }
            continue;
        }

        if event.event_type == "user" {
            if let Some(ref msg) = event.message {
                if msg.role == "user" {
                    user_events.push((event, ts));
                    continue;
                }
            }
        }

        if event.event_type == "assistant" {
            if let Some(ref msg) = event.message {
                if msg.role == "assistant" {
                    if let Some(ref msg_id) = msg.id {
                        assistant_by_id.insert(msg_id.clone(), (event, ts));
                    } else {
                        assistant_no_id.push((event, ts));
                    }
                }
            }
        }
    }

    // Build messages list
    let mut messages: Vec<ConversationMessage> = Vec::new();
    let mut tool_usage: HashMap<String, u64> = HashMap::new();
    let mut tokens = TokenTotals::zero();
    let mut lines_added: u64 = 0;
    let mut lines_removed: u64 = 0;
    let mut file_contributions: HashMap<String, FileContribution> = HashMap::new();
    let mut first_prompt = String::new();
    let mut human_lines: u64 = 0;
    let mut human_words: u64 = 0;
    let mut human_chars: u64 = 0;
    let mut model = String::new();

    // Collect all deduplicated assistant events
    let all_assistant: Vec<(RawEvent, String)> = assistant_by_id
        .into_values()
        .chain(assistant_no_id)
        .collect();

    // Merge and sort by timestamp
    let mut all_events: Vec<TaggedEvent> = Vec::new();
    for (event, ts) in user_events {
        all_events.push(TaggedEvent { kind: "user", event, ts });
    }
    for (event, ts) in all_assistant {
        all_events.push(TaggedEvent { kind: "assistant", event, ts });
    }
    all_events.sort_by(|a, b| a.ts.cmp(&b.ts));

    for tagged in &all_events {
        let msg = match &tagged.event.message {
            Some(m) => m,
            None => continue,
        };

        if tagged.kind == "user" {
            let user_text = extract_text(&msg.content);
            if first_prompt.is_empty() {
                first_prompt = user_text;
            }

            // Count human contribution
            let raw_text = extract_raw_text(&msg.content);
            let stripped = strip_html(&raw_text);
            let stripped = stripped.trim();
            if !stripped.is_empty() {
                human_lines += stripped.lines().count() as u64;
                human_words += stripped.split_whitespace().count() as u64;
                human_chars += stripped.len() as u64;
            }

            // Extract content for message thread
            let content = extract_raw_text(&msg.content);
            let content = strip_html(&content);

            messages.push(ConversationMessage {
                role: "user".to_string(),
                timestamp: tagged.ts.clone(),
                uuid: tagged.event.uuid.clone().unwrap_or_default(),
                usage: None,
                content: content.trim().to_string(),
            });
        } else {
            // Extract assistant text content (truncate to manage memory)
            let raw_content = extract_raw_text(&msg.content);
            let mut content = strip_html(&raw_content);
            if content.len() > 5000 {
                content.truncate(5000);
                content.push_str("...");
            }

            messages.push(ConversationMessage {
                role: "assistant".to_string(),
                timestamp: tagged.ts.clone(),
                uuid: tagged.event.uuid.clone().unwrap_or_default(),
                usage: msg.usage.clone(),
                content: content.trim().to_string(),
            });

            if model.is_empty() {
                if let Some(ref m) = msg.model {
                    model = m.clone();
                }
            }

            // Count tokens
            if let Some(ref usage) = msg.usage {
                tokens.input += usage.input_tokens.unwrap_or(0);
                tokens.output += usage.output_tokens.unwrap_or(0);
                tokens.cache_read += usage.cache_read_input_tokens.unwrap_or(0);
                tokens.cache_creation += usage.cache_creation_input_tokens.unwrap_or(0);
            }

            // Count tool uses and code contribution
            if let serde_json::Value::Array(ref blocks) = msg.content {
                for block in blocks {
                    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if block_type != "tool_use" {
                        continue;
                    }

                    if let Some(name) = block.get("name").and_then(|v| v.as_str()) {
                        *tool_usage.entry(name.to_string()).or_insert(0) += 1;

                        if name == "Write" {
                            if let Some(content) = block
                                .get("input")
                                .and_then(|i| i.get("content"))
                                .and_then(|c| c.as_str())
                            {
                                let lines = content.lines().count() as u64;
                                lines_added += lines;
                                if let Some(fp) = block
                                    .get("input")
                                    .and_then(|i| i.get("file_path"))
                                    .and_then(|f| f.as_str())
                                {
                                    let fc = file_contributions
                                        .entry(fp.to_string())
                                        .or_insert(FileContribution { added: 0, removed: 0 });
                                    fc.added += lines;
                                }
                            }
                        }

                        if name == "Edit" {
                            if let Some(input) = block.get("input") {
                                let old_str = input
                                    .get("old_string")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let new_str = input
                                    .get("new_string")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let old_lines = if old_str.is_empty() {
                                    0
                                } else {
                                    old_str.lines().count() as u64
                                };
                                let new_lines = if new_str.is_empty() {
                                    0
                                } else {
                                    new_str.lines().count() as u64
                                };
                                lines_removed += old_lines;
                                lines_added += new_lines;
                                if let Some(fp) = input.get("file_path").and_then(|f| f.as_str()) {
                                    let fc = file_contributions
                                        .entry(fp.to_string())
                                        .or_insert(FileContribution { added: 0, removed: 0 });
                                    fc.added += new_lines;
                                    fc.removed += old_lines;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ParsedSession {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        cwd,
        messages,
        tool_usage,
        total_tokens: tokens,
        duration_ms,
        lines_added,
        lines_removed,
        file_contributions,
        first_prompt,
        started_at,
        last_active,
        human_lines,
        human_words,
        human_chars,
        model,
        source: DataSource::Claude,
    })
}

/// Extract first text block as a single-line string
fn extract_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => {
            s.split_whitespace().collect::<Vec<_>>().join(" ")
        }
        serde_json::Value::Array(blocks) => {
            for block in blocks {
                if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        return text.split_whitespace().collect::<Vec<_>>().join(" ");
                    }
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

/// Extract raw text preserving newlines — for human contribution counting
pub fn extract_raw_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(blocks) => {
            let mut texts = Vec::new();
            for block in blocks {
                if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        texts.push(text.to_string());
                    }
                }
            }
            texts.join("\n")
        }
        _ => String::new(),
    }
}

/// Simple HTML tag stripping
pub fn strip_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

/// Quick metadata extraction: reads only first few events (does NOT read the whole file)
pub fn parse_session_metadata(file_path: &str) -> Result<(String, String)> {
    use std::io::{BufRead, BufReader};

    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut cwd = String::new();
    let mut started_at = String::new();

    for (i, line_result) in reader.lines().enumerate() {
        if i >= 20 {
            break;
        }
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let event: RawEvent = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if cwd.is_empty() {
            if let Some(ref c) = event.cwd {
                cwd = c.clone();
            }
        }
        if started_at.is_empty() {
            if let Some(ref ts) = event.timestamp {
                started_at = ts.clone();
            }
        }

        if !cwd.is_empty() && !started_at.is_empty() {
            break;
        }
    }

    Ok((cwd, started_at))
}
