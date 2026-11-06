//! Harp: the subagents a Claude Code session started, read from the files Claude Code keeps for
//! them: `<projects>/<project>/<session id>/subagents/agent-<id>.meta.json` (type, description,
//! model, and `finished` once it ends) beside `agent-<id>.jsonl`, the subagent's own transcript.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// How much of a transcript's end is read for its latest step; a step line rarely passes this.
const TAIL_BYTES: u64 = 64 * 1024;
/// How much of a transcript's end the detail view reads for its steps.
const STEPS_TAIL_BYTES: u64 = 512 * 1024;
const DETAIL_CHARS: usize = 160;
/// Without a registry entry to date the session's process, a subagent quiet this long is taken
/// as ended.
const QUIET_MEANS_ENDED: chrono::Duration = chrono::Duration::minutes(30);
/// The tool a subagent ends on when it delivers its report.
const HANDBACK_TOOL: &str = "SubagentHandback";

#[derive(Clone, Debug, PartialEq)]
pub struct Subagent {
    pub id: String,
    pub agent_type: String,
    pub description: String,
    pub model: Option<String>,
    pub background: bool,
    pub finished: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub last_active_at: Option<DateTime<Utc>>,
    /// The latest tool call, as "Read: src/main.rs".
    pub current_step: Option<String>,
    pub transcript: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StepKind {
    Tool {
        name: String,
        detail: Option<String>,
    },
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub at: Option<DateTime<Utc>>,
    pub kind: StepKind,
}

impl StepKind {
    pub fn label(&self) -> String {
        match self {
            StepKind::Tool {
                name,
                detail: Some(detail),
            } => format!("{name}: {detail}"),
            StepKind::Tool { name, detail: None } => name.clone(),
            StepKind::Text(text) => text.clone(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    agent_type: Option<String>,
    description: Option<String>,
    model: Option<String>,
    request_shape: Option<String>,
    #[serde(default)]
    finished: bool,
}

#[derive(Deserialize)]
struct Line {
    timestamp: Option<DateTime<Utc>>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    role: Option<String>,
    content: Option<serde_json::Value>,
}

/// The `subagents` folder of the session with this id, under any project in `projects_root`.
pub fn find_subagents_dir(projects_root: &Path, session_id: &str) -> Option<PathBuf> {
    fs::read_dir(projects_root)
        .ok()?
        .flatten()
        .map(|project| project.path().join(session_id).join("subagents"))
        .find(|dir| dir.is_dir())
}

/// When the running Claude Code process of this session started, from Claude Code's registry
/// (`<claude dir>/sessions/<pid>.json`). A resumed session reuses its id, so its folder also holds
/// subagents of earlier runs, none of which can still be running.
pub fn session_process_started_at(sessions_dir: &Path, session_id: &str) -> Option<DateTime<Utc>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Entry {
        session_id: String,
        started_at: i64,
    }
    fs::read_dir(sessions_dir)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| serde_json::from_slice::<Entry>(&fs::read(entry.path()).ok()?).ok())
        .filter(|entry| entry.session_id == session_id)
        .filter_map(|entry| DateTime::from_timestamp_millis(entry.started_at))
        .max()
}

/// Every subagent in `dir`, running ones first, then the most recently active. Older Claude Code
/// versions never mark a subagent finished, so one also counts as finished when it handed its
/// report back, started before the session's current process (`process_started_at`), or, with no
/// process date known, has been quiet for a while.
pub fn read_subagents(
    dir: &Path,
    process_started_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Vec<Subagent> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut subagents: Vec<Subagent> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let id = path
                .file_name()?
                .to_str()?
                .strip_prefix("agent-")?
                .strip_suffix(".meta.json")?
                .to_owned();
            let meta: Meta = serde_json::from_slice(&fs::read(&path).ok()?).ok()?;
            let transcript = dir.join(format!("agent-{id}.jsonl"));
            let tail = read_tail(&transcript, TAIL_BYTES);
            let last_line = tail.iter().rev().find(|line| line.timestamp.is_some());
            let last_active_at = last_line.and_then(|line| line.timestamp);
            let started_at = read_first_timestamp(&transcript);
            let last_tool =
                tail.iter()
                    .rev()
                    .flat_map(line_steps)
                    .find_map(|step| match step.kind {
                        StepKind::Tool { name, detail } => Some((name, detail)),
                        StepKind::Text(_) => None,
                    });
            let handed_back = last_tool
                .as_ref()
                .is_some_and(|(name, _)| name == HANDBACK_TOOL);
            let outlived = match process_started_at {
                Some(process_started_at) => started_at.is_some_and(|at| at < process_started_at),
                None => last_active_at.is_none_or(|at| now - at > QUIET_MEANS_ENDED),
            };
            Some(Subagent {
                agent_type: meta.agent_type.unwrap_or_else(|| "agent".to_owned()),
                description: meta.description.unwrap_or_default(),
                model: meta.model,
                background: meta.request_shape.as_deref() == Some("background"),
                finished: meta.finished || handed_back || outlived,
                started_at,
                last_active_at,
                current_step: last_tool
                    .map(|(name, detail)| StepKind::Tool { name, detail }.label()),
                id,
                transcript,
            })
        })
        .collect();
    subagents.sort_by(|a, b| {
        a.finished
            .cmp(&b.finished)
            .then(b.last_active_at.cmp(&a.last_active_at))
    });
    subagents
}

/// The last `limit` steps of a subagent's transcript, oldest first.
pub fn read_steps(transcript: &Path, limit: usize) -> Vec<Step> {
    let steps: Vec<Step> = read_tail(transcript, STEPS_TAIL_BYTES)
        .iter()
        .flat_map(line_steps)
        .collect();
    steps[steps.len().saturating_sub(limit)..].to_vec()
}

fn line_steps(line: &Line) -> Vec<Step> {
    let Some(Message {
        role: Some(role),
        content: Some(serde_json::Value::Array(blocks)),
    }) = &line.message
    else {
        return Vec::new();
    };
    if role != "assistant" {
        return Vec::new();
    }
    blocks
        .iter()
        .filter_map(|block| {
            let kind = match block.get("type")?.as_str()? {
                "tool_use" => StepKind::Tool {
                    name: block.get("name")?.as_str()?.to_owned(),
                    detail: block.get("input").and_then(tool_detail),
                },
                "text" => StepKind::Text(clip(block.get("text")?.as_str()?.trim())?),
                _ => return None,
            };
            Some(Step {
                at: line.timestamp,
                kind,
            })
        })
        .collect()
}

/// The input field that says what a tool call is about, in the order people scan for it.
fn tool_detail(input: &serde_json::Value) -> Option<String> {
    [
        "description",
        "command",
        "file_path",
        "path",
        "pattern",
        "url",
        "query",
        "prompt",
    ]
    .iter()
    .find_map(|key| input.get(*key)?.as_str())
    .and_then(clip)
}

fn clip(text: &str) -> Option<String> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return None;
    }
    Some(match text.char_indices().nth(DETAIL_CHARS) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text,
    })
}

/// The parsable lines in the last `bytes` of a JSONL file; a line cut by the window is skipped.
fn read_tail(path: &Path, bytes: u64) -> Vec<Line> {
    let Ok(mut file) = File::open(path) else {
        return Vec::new();
    };
    let len = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let start = len.saturating_sub(bytes);
    let mut bytes_read = Vec::new();
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_to_end(&mut bytes_read).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes_read);
    let text = if start > 0 {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        &text
    };
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn read_first_timestamp(path: &Path) -> Option<DateTime<Utc>> {
    let mut head = Vec::new();
    File::open(path)
        .ok()?
        .take(TAIL_BYTES)
        .read_to_end(&mut head)
        .ok()?;
    String::from_utf8_lossy(&head)
        .lines()
        .filter_map(|line| serde_json::from_str::<Line>(line).ok())
        .find_map(|line| line.timestamp)
}

#[cfg(test)]
#[path = "subagents_tests.rs"]
mod tests;
