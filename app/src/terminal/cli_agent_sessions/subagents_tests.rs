use std::fs;
use std::path::Path;

use super::{StepKind, find_subagents_dir, read_steps, read_subagents};

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

    let agents = read_subagents(&dir);
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
