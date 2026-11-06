use std::fs;
use std::path::Path;

use chrono::{DateTime, Duration, Utc};

use super::{StepKind, find_subagents_dir, read_steps, read_subagents, session_process_started_at};

fn write_agent(dir: &Path, id: &str, meta: &str, lines: &[&str]) {
    fs::write(dir.join(format!("agent-{id}.meta.json")), meta).unwrap();
    fs::write(
        dir.join(format!("agent-{id}.jsonl")),
        lines.join("\n") + "\n",
    )
    .unwrap();
}

fn tool_line(at: &str, name: &str, input: &str) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"{at}","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"t","name":"{name}","input":{input}}}]}}}}"#
    )
}

#[test]
fn reads_running_and_finished_subagents_with_their_latest_step() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("-Users-me-repo/sess-1/subagents");
    fs::create_dir_all(&dir).unwrap();
    let user = r#"{"type":"user","timestamp":"2026-11-06T20:15:00Z","message":{"role":"user","content":"Go"}}"#;
    write_agent(
        &dir,
        "aaa",
        r#"{"agentType":"Explore","description":"Probe the panel","requestShape":"background","model":"haiku"}"#,
        &[
            user,
            &tool_line(
                "2026-11-06T20:15:05Z",
                "Read",
                r#"{"file_path":"/repo/a.rs","limit":40}"#,
            ),
            &tool_line(
                "2026-11-06T20:15:09Z",
                "Bash",
                r#"{"command":"cargo   test -p warp"}"#,
            ),
        ],
    );
    write_agent(
        &dir,
        "bbb",
        r#"{"agentType":"general-purpose","description":"Old one","requestShape":"foreground","finished":true}"#,
        &[
            user,
            &tool_line("2026-11-06T20:15:01Z", "Grep", r#"{"pattern":"fn main"}"#),
        ],
    );

    assert_eq!(find_subagents_dir(root.path(), "sess-1"), Some(dir.clone()));
    assert_eq!(find_subagents_dir(root.path(), "missing"), None);

    let now: DateTime<Utc> = "2026-11-06T20:16:00Z".parse().unwrap();
    let agents = read_subagents(&dir, None, now);
    assert_eq!(agents.len(), 2);
    let running = &agents[0];
    assert_eq!(running.id, "aaa");
    assert!(!running.finished && running.background);
    assert_eq!(running.agent_type, "Explore");
    assert_eq!(running.model.as_deref(), Some("haiku"));
    assert_eq!(
        running.current_step.as_deref(),
        Some("Bash: cargo test -p warp")
    );
    assert_eq!(
        running.started_at.map(|at| at.to_rfc3339()),
        Some("2026-11-06T20:15:00+00:00".to_owned())
    );
    assert_eq!(
        running.last_active_at.map(|at| at.to_rfc3339()),
        Some("2026-11-06T20:15:09+00:00".to_owned())
    );
    assert!(agents[1].finished && !agents[1].background);
}

#[test]
fn steps_list_tool_calls_and_text_oldest_first() {
    let root = tempfile::tempdir().unwrap();
    let text = r#"{"type":"assistant","timestamp":"2026-11-06T20:16:00Z","message":{"role":"assistant","content":[{"type":"thinking","thinking":"x"},{"type":"text","text":"  Found 3 issues.  "}]}}"#;
    write_agent(
        root.path(),
        "ccc",
        "{}",
        &[
            &tool_line(
                "2026-11-06T20:15:05Z",
                "Read",
                r#"{"file_path":"/repo/a.rs"}"#,
            ),
            &tool_line(
                "2026-11-06T20:15:06Z",
                "WebFetch",
                r#"{"url":"https://x.dev","prompt":"p"}"#,
            ),
            text,
            "not json",
        ],
    );
    let transcript = root.path().join("agent-ccc.jsonl");

    let labels: Vec<String> = read_steps(&transcript, 10)
        .iter()
        .map(|step| step.kind.label())
        .collect();
    assert_eq!(
        labels,
        [
            "Read: /repo/a.rs",
            "WebFetch: https://x.dev",
            "Found 3 issues."
        ]
    );

    let last_two = read_steps(&transcript, 2);
    assert_eq!(last_two.len(), 2);
    assert!(matches!(last_two[1].kind, StepKind::Text(_)));
}

#[test]
fn old_runs_handbacks_and_quiet_agents_count_as_finished() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path();
    let user = |at: &str| {
        format!(
            r#"{{"type":"user","timestamp":"{at}","message":{{"role":"user","content":"Go"}}}}"#
        )
    };
    let meta = r#"{"agentType":"general-purpose","description":"d","requestShape":"background"}"#;
    // Ran in an earlier process of a resumed session; never marked finished.
    write_agent(
        dir,
        "old",
        meta,
        &[
            &user("2026-10-16T15:00:00Z"),
            &tool_line("2026-10-16T15:01:00Z", "Read", "{}"),
        ],
    );
    // Delivered its report, then the parent's tool result came back.
    write_agent(
        dir,
        "done",
        meta,
        &[
            &user("2026-11-06T20:00:00Z"),
            &tool_line(
                "2026-11-06T20:05:00Z",
                "SubagentHandback",
                r#"{"message":"x"}"#,
            ),
            &user("2026-11-06T20:05:01Z"),
        ],
    );
    // Started in the current process and still working.
    write_agent(
        dir,
        "live",
        meta,
        &[
            &user("2026-11-06T20:10:00Z"),
            &tool_line(
                "2026-11-06T20:14:00Z",
                "Bash",
                r#"{"command":"cargo build"}"#,
            ),
        ],
    );

    let now: DateTime<Utc> = "2026-11-06T20:15:00Z".parse().unwrap();
    let process_started = Some(now - Duration::hours(1));
    let running: Vec<String> = read_subagents(dir, process_started, now)
        .into_iter()
        .filter(|agent| !agent.finished)
        .map(|agent| agent.id)
        .collect();
    assert_eq!(running, ["live"]);

    // No registry entry: only recent activity counts as running.
    let later = now + Duration::hours(2);
    assert!(
        read_subagents(dir, None, later)
            .iter()
            .all(|agent| agent.finished)
    );
}

#[test]
fn registry_entry_dates_the_sessions_current_process() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("111.json"),
        r#"{"pid":111,"sessionId":"s-1","startedAt":1793997477272,"status":"idle"}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("222.json"),
        r#"{"pid":222,"sessionId":"s-2","startedAt":1}"#,
    )
    .unwrap();
    fs::write(dir.path().join("111.abc.key"), "not json").unwrap();

    assert_eq!(
        session_process_started_at(dir.path(), "s-1").map(|at| at.timestamp_millis()),
        Some(1793997477272)
    );
    assert_eq!(session_process_started_at(dir.path(), "missing"), None);
}
