use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::models::{DataSource, ScannedProject, SessionFile};
use crate::parser::parse_session_metadata;

pub fn get_projects_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".claude")
        .join("projects")
}

pub fn scan_claude_projects() -> Result<Vec<ScannedProject>> {
    let projects_dir = get_projects_dir();

    if !projects_dir.exists() {
        return Ok(vec![]);
    }

    let mut projects = Vec::new();

    let entries = fs::read_dir(&projects_dir)?;
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let project_dir = entry.path();
        let project_id = entry.file_name().to_string_lossy().to_string();

        let mut session_files = Vec::new();
        if let Ok(files) = fs::read_dir(&project_dir) {
            for f in files {
                let f = f?;
                let fname = f.file_name().to_string_lossy().to_string();
                if !fname.ends_with(".jsonl") || f.file_type()?.is_dir() {
                    continue;
                }
                let meta = f.metadata()?;
                session_files.push(SessionFile {
                    id: fname.trim_end_matches(".jsonl").to_string(),
                    path: f.path().to_string_lossy().to_string(),
                    size: meta.len(),
                    source: DataSource::Claude,
                });
            }
        }

        if !session_files.is_empty() {
            // Resolve real project path from session cwd metadata
            let real_dir = resolve_claude_project_dir(&session_files, &project_id);

            projects.push(ScannedProject {
                id: project_id,
                dir: real_dir,
                source: DataSource::Claude,
                sources: vec![DataSource::Claude],
                session_files,
            });
        }
    }

    Ok(projects)
}

/// Resolve a Claude project's actual filesystem path.
/// Reads the first session file's cwd, falls back to decoding the project ID.
fn resolve_claude_project_dir(session_files: &[SessionFile], project_id: &str) -> String {
    // Try reading cwd from the first (or most recent by name) session file
    // Session files are UUIDs, so just try the first one
    for sf in session_files.iter().take(3) {
        if let Ok((cwd, _)) = parse_session_metadata(&sf.path) {
            if !cwd.is_empty() {
                return cwd;
            }
        }
    }

    // Fallback: decode project ID (e.g. "-Users-adrian-Projects-foo" -> try "/Users/adrian/Projects/foo")
    decode_project_id(project_id)
}

/// Decode a Claude project ID back to a filesystem path.
/// The encoding replaces '/' with '-', so we check if the decoded path exists.
fn decode_project_id(id: &str) -> String {
    // Try common path prefixes
    let path_with_slashes = format!("/{}", id.trim_start_matches('-').replace('-', "/"));

    // Walk backwards and check if path exists, handling folder names with dashes
    // Try the full replacement first
    if std::path::Path::new(&path_with_slashes).exists() {
        return path_with_slashes;
    }

    // If that doesn't work, return the encoded id as-is (won't merge, but won't crash)
    id.to_string()
}

/// Merge Claude and Cursor projects by resolved filesystem path.
/// Projects sharing the same path get merged into one with sources = [Claude, Cursor].
pub fn scan_all_projects(
    claude_projects: Vec<ScannedProject>,
    cursor_projects: Vec<ScannedProject>,
) -> Vec<ScannedProject> {
    // Build a map keyed by resolved path
    let mut by_path: HashMap<String, ScannedProject> = HashMap::new();

    for proj in claude_projects {
        let key = normalize_path(&proj.dir);
        by_path.insert(key, proj);
    }

    for cursor_proj in cursor_projects {
        let key = normalize_path(&cursor_proj.dir);
        if let Some(existing) = by_path.get_mut(&key) {
            // Merge: add cursor sessions + update sources
            existing.session_files.extend(cursor_proj.session_files);
            if !existing.sources.contains(&DataSource::Cursor) {
                existing.sources.push(DataSource::Cursor);
            }
        } else {
            by_path.insert(key, cursor_proj);
        }
    }

    by_path.into_values().collect()
}

fn normalize_path(path: &str) -> String {
    path.trim_end_matches('/').to_string()
}
