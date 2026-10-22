//! Harp: which Claude Code session is live in each terminal pane, kept across restarts in
//! `<data dir>/agent_sessions.json` so a restored pane can run `claude --resume <id>`.
//! Each save of a pane's snapshot syncs its entry, so the file mirrors what a restore rebuilds.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Pane uuid (lowercase hex) to the Claude session id live in that pane.
type Sessions = BTreeMap<String, String>;

fn path() -> PathBuf {
    warp_core::paths::data_dir().join("agent_sessions.json")
}

fn sessions() -> &'static Mutex<Sessions> {
    static SESSIONS: OnceLock<Mutex<Sessions>> = OnceLock::new();
    SESSIONS.get_or_init(|| {
        Mutex::new(
            std::fs::read_to_string(path())
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default(),
        )
    })
}

fn key(pane_uuid: &[u8]) -> String {
    pane_uuid.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Records the Claude session live in a pane, or forgets the pane's entry with `None`. The file
/// is rewritten only when the entry changes, since snapshots are taken on every app save.
pub fn record(pane_uuid: &[u8], session_id: Option<&str>) {
    let mut sessions = sessions().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let key = key(pane_uuid);
    let changed = match session_id {
        Some(id) => sessions.insert(key, id.to_owned()).as_deref() != Some(id),
        None => sessions.remove(&key).is_some(),
    };
    if !changed {
        return;
    }
    match serde_json::to_string_pretty(&*sessions) {
        Ok(text) => {
            if let Err(err) = std::fs::write(path(), text) {
                log::warn!("Could not write agent_sessions.json: {err}");
            }
        }
        Err(err) => log::warn!("Could not encode agent sessions: {err}"),
    }
}

/// The Claude session that was live in a pane at its last save. Only ids made of letters, digits
/// and dashes come back, because the id is pasted into a shell command.
pub fn session_for(pane_uuid: &[u8]) -> Option<String> {
    let sessions = sessions().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    sessions
        .get(&key(pane_uuid))
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .cloned()
}
