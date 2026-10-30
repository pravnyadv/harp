use std::collections::HashMap;

use super::{Activity, Session, ancestors, session_under};

fn parents(pairs: &[(u32, u32)]) -> impl FnMut(u32) -> Option<u32> {
    let parents: HashMap<u32, u32> = pairs.iter().copied().collect();
    move |pid| parents.get(&pid).copied()
}

#[test]
fn claude_started_from_the_shell_descends_from_it() {
    assert!(ancestors(200, parents(&[(200, 100), (100, 1)])).contains(&100));
}

#[test]
fn claude_started_through_a_wrapper_script_descends_from_the_shell() {
    assert!(ancestors(300, parents(&[(300, 200), (200, 100), (100, 1)])).contains(&100));
}

#[test]
fn claude_in_another_pane_does_not_descend_from_the_shell() {
    assert!(!ancestors(200, parents(&[(200, 150), (150, 1)])).contains(&100));
}

#[test]
fn ancestry_deeper_than_the_limit_is_not_followed() {
    let chain = [(6, 5), (5, 4), (4, 3), (3, 2), (2, 1)];
    assert!(!ancestors(6, parents(&chain)).contains(&1));
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

fn activity(json_fields: &str) -> Option<(Activity, u64)> {
    serde_json::from_str::<Session>(&format!(r#"{{"pid":1,"sessionId":"s",{json_fields}}}"#))
        .unwrap()
        .activity()
}

#[test]
fn a_running_turn_reads_busy() {
    assert_eq!(
        activity(r#""status":"busy","statusUpdatedAt":2000"#),
        Some((Activity::Busy, 2_000))
    );
}

#[test]
fn a_dialog_reads_waiting_with_its_label() {
    assert_eq!(
        activity(r#""status":"waiting","waitingFor":"permission prompt","statusUpdatedAt":5"#),
        Some((Activity::Waiting(Some("permission prompt".to_owned())), 5))
    );
}

#[test]
fn background_shells_after_the_turn_read_idle() {
    assert_eq!(
        activity(r#""status":"shell","statusUpdatedAt":7"#),
        Some((Activity::Idle, 7))
    );
}

#[test]
fn an_unknown_or_undated_status_says_nothing() {
    assert_eq!(
        activity(r#""status":"compacting","statusUpdatedAt":7"#),
        None
    );
    assert_eq!(activity(r#""status":"busy""#), None);
    assert_eq!(activity(r#""waitingFor":"x""#), None);
}

#[test]
fn a_shell_with_no_claude_under_it_has_no_session() {
    assert!(session_under(std::process::id()).is_none());
}
