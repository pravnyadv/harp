//! Harp: Claude Code's own registry of running sessions, `~/.claude/sessions/<pid>.json`. Hook
//! events name a pane's session only once Claude sends one, so a session that never reported in
//! during this app run (one switched to with `/resume`, say) is only found here.

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
    /// "busy", "idle", "shell", or "waiting" while a dialog asks the user.
    status: Option<String>,
    /// When `status` last changed (Unix ms).
    status_updated_at: Option<u64>,
}

impl Session {
    /// Whether the dialog Claude waited on has closed since `since_ms`. No hook reports an
    /// answered permission prompt, so this file is the only early sign of one.
    pub fn left_dialog_since(&self, since_ms: u64) -> bool {
        self.status.as_deref() != Some("waiting")
            && self.status_updated_at.is_some_and(|at| at > since_ms)
    }
}

/// The Claude process running under the shell `shell_pid`, if Claude registered it. A registry
/// file whose pid now belongs to some other program is skipped.
pub fn session_under(shell_pid: u32) -> Option<Session> {
    let dir = dirs::home_dir()?.join(".claude/sessions");
    let mut system = System::new();
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|file| file.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|file| {
            serde_json::from_str::<Session>(&std::fs::read_to_string(file.path()).ok()?).ok()
        })
        .find(|entry| {
            process(&mut system, entry.pid).is_some_and(|process| process.name() == "claude")
                && descends_from(entry.pid, shell_pid, |pid| {
                    process(&mut system, pid)?.parent().map(Pid::as_u32)
                })
        })
}

fn process(system: &mut System, pid: u32) -> Option<&Process> {
    let pid = Pid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid)
}

fn descends_from(pid: u32, ancestor: u32, mut parent_of: impl FnMut(u32) -> Option<u32>) -> bool {
    let mut current = pid;
    for _ in 0..MAX_DEPTH {
        match parent_of(current) {
            Some(parent) if parent == ancestor => return true,
            Some(parent) => current = parent,
            None => return false,
        }
    }
    false
}

#[cfg(test)]
#[path = "claude_registry_tests.rs"]
mod tests;
