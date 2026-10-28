//! Harp: which Claude Code session is live in each terminal pane, kept across restarts in
//! `<data dir>/agent_sessions.json` so a restored pane can run `claude --resume <id>`.
//! Each save of a pane's snapshot syncs its entry, so the file mirrors what a restore rebuilds.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Default)]
struct Store {
    /// Pane uuid (lowercase hex) to the Claude session id live in that pane.
    sessions: BTreeMap<String, String>,
    /// Panes queued to resume whose Claude has not reported a session id yet. Saves taken in
    /// that window see no live session, and must not erase the id the resume depends on.
    awaiting_resume: HashSet<String>,
}

impl Store {
    /// Returns whether the stored sessions changed.
    fn record(&mut self, key: String, session_id: Option<&str>) -> bool {
        match session_id {
            Some(id) => {
                self.awaiting_resume.remove(&key);
                self.sessions.insert(key, id.to_owned()).as_deref() != Some(id)
            }
            None if self.awaiting_resume.contains(&key) => false,
            None => self.sessions.remove(&key).is_some(),
        }
    }

    fn take_for_resume(&mut self, key: String) -> Option<String> {
        let session_id = self
            .sessions
            .get(&key)
            .filter(|id| {
                !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            })
            .cloned()?;
        self.awaiting_resume.insert(key);
        Some(session_id)
    }
}

fn path() -> PathBuf {
    warp_core::paths::data_dir().join("agent_sessions.json")
}

fn store() -> MutexGuard<'static, Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE
        .get_or_init(|| {
            Mutex::new(Store {
                sessions: std::fs::read_to_string(path())
                    .ok()
                    .and_then(|text| serde_json::from_str(&text).ok())
                    .unwrap_or_default(),
                awaiting_resume: HashSet::new(),
            })
        })
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn key(pane_uuid: &[u8]) -> String {
    pane_uuid.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Records the Claude session live in a pane, or forgets the pane's entry with `None`. The file
/// is rewritten only when the entry changes, since snapshots are taken on every app save.
pub fn record(pane_uuid: &[u8], session_id: Option<&str>) {
    let mut store = store();
    if !store.record(key(pane_uuid), session_id) {
        return;
    }
    match serde_json::to_string_pretty(&store.sessions) {
        Ok(text) => {
            if let Err(err) = std::fs::write(path(), text) {
                log::warn!("Could not write agent_sessions.json: {err}");
            }
        }
        Err(err) => log::warn!("Could not encode agent sessions: {err}"),
    }
}

/// The Claude session that was live in a pane at its last save, for a restore about to resume
/// it; the entry is kept until that Claude reports in. Only ids made of letters, digits and
/// dashes come back, because the id is pasted into a shell command.
pub fn take_for_resume(pane_uuid: &[u8]) -> Option<String> {
    store().take_for_resume(key(pane_uuid))
}

#[cfg(test)]
#[path = "resume_store_tests.rs"]
mod tests;
