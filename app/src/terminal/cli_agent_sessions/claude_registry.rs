//! Harp: Claude Code's own registry of running sessions, `~/.claude/sessions/<pid>.json`. Hook
//! events name a pane's session only once Claude sends one, so a session that never reported in
//! during this app run (one switched to with `/resume`, say) is only found here.

use std::collections::HashMap;

use sysinfo::{Pid, Process, ProcessesToUpdate, System};

/// How far above a Claude process its pane's shell may sit; `~/.claude/bin/handoff-open` puts a
/// bash process in between.
const MAX_DEPTH: usize = 4;

/// A running Claude Code's registry entry.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pid: u32,
    pub session_id: String,
    /// "busy", "idle", "shell" (idle with background shells), or "waiting" while a dialog asks
    /// the user.
    status: Option<String>,
    /// What a "waiting" session waits on, e.g. "permission prompt".
    waiting_for: Option<String>,
    /// When `status` last changed (Unix ms).
    status_updated_at: Option<u64>,
}

/// What a Claude Code session is doing, by its own account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Activity {
    Busy,
    /// A dialog waits for the user; the label names it.
    Waiting(Option<String>),
    /// No turn runs, though background shells may.
    Idle,
}

impl Session {
    /// What Claude says it is doing, and since when (Unix ms). `None` for a status Harp does not
    /// know.
    pub fn activity(&self) -> Option<(Activity, u64)> {
        let activity = match self.status.as_deref()? {
            "busy" => Activity::Busy,
            "waiting" => Activity::Waiting(self.waiting_for.clone()),
            "idle" | "shell" => Activity::Idle,
            _ => return None,
        };
        Some((activity, self.status_updated_at?))
    }
}

/// The Claude process running under the shell `shell_pid`, if Claude registered it.
pub fn session_under(shell_pid: u32) -> Option<Session> {
    sessions_under(&[shell_pid]).remove(&shell_pid)
}

/// The Claude process running under each of `shell_pids`, keyed by shell pid, from one read of
/// the registry. A registry file whose pid now belongs to some other program is skipped, and of
/// two Claudes under one shell (one started by the other's tool) the outer one wins.
pub fn sessions_under(shell_pids: &[u32]) -> HashMap<u32, Session> {
    let Some(dir) = dirs::home_dir().map(|home| home.join(".claude/sessions")) else {
        return HashMap::new();
    };
    let Ok(files) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    let mut system = System::new();
    let mut found: HashMap<u32, (usize, Session)> = HashMap::new();
    for entry in files
        .filter_map(Result::ok)
        .filter(|file| file.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|file| {
            serde_json::from_str::<Session>(&std::fs::read_to_string(file.path()).ok()?).ok()
        })
    {
        if process(&mut system, entry.pid).is_none_or(|process| process.name() != "claude") {
            continue;
        }
        let ancestors = ancestors(entry.pid, |pid| {
            process(&mut system, pid)?.parent().map(Pid::as_u32)
        });
        let Some((depth, shell)) = ancestors
            .into_iter()
            .enumerate()
            .find(|(_, pid)| shell_pids.contains(pid))
        else {
            continue;
        };
        if found
            .get(&shell)
            .is_none_or(|(nearest, _)| depth < *nearest)
        {
            found.insert(shell, (depth, entry));
        }
    }
    found
        .into_iter()
        .map(|(shell, (_, session))| (shell, session))
        .collect()
}

fn process(system: &mut System, pid: u32) -> Option<&Process> {
    let pid = Pid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid)
}

/// `pid`'s parent, grandparent and so on, at most `MAX_DEPTH` of them.
fn ancestors(pid: u32, mut parent_of: impl FnMut(u32) -> Option<u32>) -> Vec<u32> {
    std::iter::successors(parent_of(pid), |&parent| parent_of(parent))
        .take(MAX_DEPTH)
        .collect()
}

#[cfg(test)]
#[path = "claude_registry_tests.rs"]
mod tests;
