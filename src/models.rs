use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Data source enum ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataSource {
    Claude,
    Cursor,
}

impl DataSource {
    pub fn label(self) -> &'static str {
        match self {
            DataSource::Claude => "Claude",
            DataSource::Cursor => "Cursor",
        }
    }
}

impl std::fmt::Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// --- Raw JSONL event types ---

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RawEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub uuid: Option<String>,
    pub timestamp: Option<String>,
    pub message: Option<RawMessage>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct RawMessage {
    pub role: String,
    pub content: serde_json::Value, // string | ContentBlock[]
    pub model: Option<String>,
    pub id: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

// --- Processed types ---

#[derive(Debug, Clone, Serialize)]
pub struct TokenTotals {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
}

impl TokenTotals {
    pub fn zero() -> Self {
        Self { input: 0, output: 0, cache_read: 0, cache_creation: 0 }
    }

    pub fn total(&self) -> u64 {
        self.input + self.output
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FileContribution {
    pub added: u64,
    pub removed: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationMessage {
    pub role: String,
    pub timestamp: String,
    pub uuid: String,
    pub usage: Option<TokenUsage>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedSession {
    pub session_id: String,
    pub project_id: String,
    pub cwd: String,
    pub messages: Vec<ConversationMessage>,
    pub tool_usage: HashMap<String, u64>,
    pub total_tokens: TokenTotals,
    pub duration_ms: f64,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub file_contributions: HashMap<String, FileContribution>,
    pub first_prompt: String,
    pub started_at: String,
    pub last_active: String,
    pub human_lines: u64,
    pub human_words: u64,
    pub human_chars: u64,
    pub model: String,
    pub source: DataSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionFile {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub source: DataSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScannedProject {
    pub id: String,
    pub dir: String,
    pub source: DataSource,
    pub sources: Vec<DataSource>,
    pub session_files: Vec<SessionFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub session_count: usize,
    pub message_count: usize,
    pub total_tokens: TokenTotals,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub last_active: String,
    pub tool_usage: HashMap<String, u64>,
    pub cost: f64,
    pub model: String,
    pub sessions: Vec<ParsedSession>,
    pub sources: Vec<DataSource>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEntry {
    pub date: String,
    pub sessions: u64,
    pub messages: u64,
    pub token_input: u64,
    pub token_output: u64,
    pub claude_sessions: u64,
    pub cursor_sessions: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GlobalMetrics {
    pub total_projects: usize,
    pub total_sessions: usize,
    pub total_messages: usize,
    pub total_tokens: TokenTotals,
    pub tool_usage: HashMap<String, u64>,
    pub timeline: Vec<TimelineEntry>,
    pub total_lines_added: u64,
    pub total_lines_removed: u64,
    pub total_cost: f64,
    pub human_lines: u64,
    pub human_words: u64,
    pub human_chars: u64,
}

impl GlobalMetrics {
    pub fn empty() -> Self {
        Self {
            total_projects: 0,
            total_sessions: 0,
            total_messages: 0,
            total_tokens: TokenTotals::zero(),
            tool_usage: HashMap::new(),
            timeline: Vec::new(),
            total_lines_added: 0,
            total_lines_removed: 0,
            total_cost: 0.0,
            human_lines: 0,
            human_words: 0,
            human_chars: 0,
        }
    }
}
