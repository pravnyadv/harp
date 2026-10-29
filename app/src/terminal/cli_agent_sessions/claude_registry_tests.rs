use std::collections::HashMap;

use super::{Session, descends_from, session_under};

fn parents(pairs: &[(u32, u32)]) -> impl FnMut(u32) -> Option<u32> {
    let parents: HashMap<u32, u32> = pairs.iter().copied().collect();
    move |pid| parents.get(&pid).copied()
}

#[test]
fn claude_started_from_the_shell_descends_from_it() {
    assert!(descends_from(200, 100, parents(&[(200, 100), (100, 1)])));
}

#[test]
fn claude_started_through_a_wrapper_script_descends_from_the_shell() {
    assert!(descends_from(
        300,
        100,
        parents(&[(300, 200), (200, 100), (100, 1)])
    ));
}

#[test]
fn claude_in_another_pane_does_not_descend_from_the_shell() {
    assert!(!descends_from(200, 100, parents(&[(200, 150), (150, 1)])));
}

#[test]
fn ancestry_deeper_than_the_limit_is_not_followed() {
    let chain = [(6, 5), (5, 4), (4, 3), (3, 2), (2, 1)];
    assert!(!descends_from(6, 1, parents(&chain)));
}

#[test]
fn registry_entries_parse_from_claude_code_files() {
    let entry: Session = serde_json::from_str(
        r#"{"pid":49297,"sessionId":"c90a34e6-dc8b-4f44-afdf-aaed71270f22","procStart":"Wed Oct 28 19:48:59 2026","pidDomain":"darwin","kind":"interactive","status":"idle"}"#,
    )
    .unwrap();
    assert_eq!(entry.pid, 49297);
    assert_eq!(entry.session_id, "c90a34e6-dc8b-4f44-afdf-aaed71270f22");
}

fn session(status: &str, status_updated_at: u64) -> Session {
    serde_json::from_str(&format!(
        r#"{{"pid":1,"sessionId":"s","status":"{status}","statusUpdatedAt":{status_updated_at}}}"#
    ))
    .unwrap()
}

#[test]
fn a_dialog_answered_after_it_was_reported_has_closed() {
    assert!(session("busy", 2_000).left_dialog_since(1_000));
}

#[test]
fn a_dialog_still_waiting_has_not_closed() {
    assert!(!session("waiting", 2_000).left_dialog_since(1_000));
}

#[test]
fn work_from_before_the_dialog_was_reported_does_not_close_it() {
    // Claude may report the dialog before its registry turns "waiting".
    assert!(!session("busy", 500).left_dialog_since(1_000));
}

#[test]
fn an_entry_without_a_status_never_closes_a_dialog() {
    let entry: Session = serde_json::from_str(r#"{"pid":1,"sessionId":"s"}"#).unwrap();
    assert!(!entry.left_dialog_since(0));
}

#[test]
fn a_shell_with_no_claude_under_it_has_no_session() {
    assert!(session_under(std::process::id()).is_none());
}
