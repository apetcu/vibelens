use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::models::{DataSource, ScannedProject, SessionFile};

fn cursor_workspace_storage_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Library")
        .join("Application Support")
        .join("Cursor")
        .join("User")
        .join("workspaceStorage")
}

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
struct ComposerHead {
    #[serde(rename = "composerId")]
    composer_id: String,
    #[serde(rename = "createdAt")]
    #[allow(dead_code)]
    created_at: Option<f64>,
    #[serde(rename = "isArchived")]
    is_archived: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct ComposerData {
    #[serde(rename = "allComposers")]
    all_composers: Option<Vec<ComposerHead>>,
}

#[derive(Debug, serde::Deserialize)]
struct WorkspaceJson {
    folder: Option<String>,
}

/// Get composer IDs that have actual bubble messages in the global DB.
fn get_composer_ids_with_bubbles() -> HashSet<String> {
    let db_path = cursor_global_db_path();
    if !db_path.exists() {
        return HashSet::new();
    }

    let conn = match Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(_) => return HashSet::new(),
    };

    // Keys are formatted as bubbleId:<composerId>:<bubbleId>
    // Extract distinct composer IDs (chars 10-46 = 36-char UUID at index 9..45)
    let mut stmt = match conn
        .prepare("SELECT DISTINCT substr(key, 10, 36) AS cid FROM cursorDiskKV WHERE key LIKE 'bubbleId:%'")
    {
        Ok(s) => s,
        Err(_) => return HashSet::new(),
    };

    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap_or_else(|_| panic!("query failed"))
        .filter_map(|r| r.ok())
        .collect();

    ids.into_iter().collect()
}

/// Scan all Cursor workspace directories and return projects with session files.
pub fn scan_cursor_projects() -> Result<Vec<ScannedProject>> {
    let storage_dir = cursor_workspace_storage_dir();
    if !storage_dir.exists() {
        return Ok(vec![]);
    }

    let active_composers = get_composer_ids_with_bubbles();
    let mut projects = Vec::new();

    let entries = match fs::read_dir(&storage_dir) {
        Ok(e) => e,
        Err(_) => return Ok(vec![]),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        let workspace_dir = entry.path();
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let db_path = workspace_dir.join("state.vscdb");
        let workspace_json_path = workspace_dir.join("workspace.json");

        // Check state.vscdb exists
        if !db_path.exists() {
            continue;
        }

        // Read workspace.json to get project folder path
        let project_folder = match fs::read_to_string(&workspace_json_path) {
            Ok(raw) => {
                let ws: WorkspaceJson = match serde_json::from_str(&raw) {
                    Ok(w) => w,
                    Err(_) => continue,
                };
                match ws.folder {
                    Some(folder) => {
                        // Skip remote workspaces
                        if folder.starts_with("vscode-remote://") {
                            continue;
                        }
                        // Decode file:// URI to path
                        let path = folder
                            .strip_prefix("file://")
                            .unwrap_or(&folder);
                        url_decode(path)
                    }
                    None => continue,
                }
            }
            Err(_) => continue,
        };

        if project_folder.is_empty() {
            continue;
        }

        // Read composer data from SQLite
        let composers = match read_composers(&db_path) {
            Some(c) => c,
            None => continue,
        };

        // Filter to non-archived composers that have actual bubbles
        let active: Vec<&ComposerHead> = composers
            .iter()
            .filter(|c| !c.is_archived.unwrap_or(false))
            .filter(|c| active_composers.contains(&c.composer_id))
            .collect();

        if active.is_empty() {
            continue;
        }

        let project_id = format!("cursor-{}", dir_name);

        let session_files: Vec<SessionFile> = active
            .iter()
            .map(|c| SessionFile {
                id: c.composer_id.clone(),
                path: db_path.to_string_lossy().to_string(),
                size: 0, // not meaningful for SQLite-backed sessions
                source: DataSource::Cursor,
            })
            .collect();

        projects.push(ScannedProject {
            id: project_id,
            dir: project_folder,
            source: DataSource::Cursor,
            sources: vec![DataSource::Cursor],
            session_files,
        });
    }

    Ok(projects)
}

fn read_composers(db_path: &PathBuf) -> Option<Vec<ComposerHead>> {
    let conn = Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;

    // Try ItemTable first
    if let Some(composers) = query_composer_data(&conn, "ItemTable") {
        if !composers.is_empty() {
            return Some(composers);
        }
    }

    // Fall back to cursorDiskKV
    query_composer_data(&conn, "cursorDiskKV")
}

fn query_composer_data(conn: &Connection, table: &str) -> Option<Vec<ComposerHead>> {
    let sql = format!(
        "SELECT value FROM {} WHERE key = 'composer.composerData'",
        table
    );
    let row: Option<String> = conn
        .query_row(&sql, [], |row| row.get(0))
        .ok();

    if let Some(value) = row {
        let data: ComposerData = serde_json::from_str(&value).ok()?;
        Some(data.all_composers.unwrap_or_default())
    } else {
        None
    }
}

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }
    result
}
