use std::collections::HashMap;

use super::{Entry, descends_from, session_under};

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
    let entry: Entry = serde_json::from_str(
        r#"{"pid":49297,"sessionId":"c90a34e6-dc8b-4f44-afdf-aaed71270f22","procStart":"Wed Oct 28 19:48:59 2026","pidDomain":"darwin","kind":"interactive","status":"idle"}"#,
    )
    .unwrap();
    assert_eq!(entry.pid, 49297);
    assert_eq!(entry.session_id, "c90a34e6-dc8b-4f44-afdf-aaed71270f22");
}

#[test]
fn a_shell_with_no_claude_under_it_has_no_session() {
    assert_eq!(session_under(std::process::id()), None);
}
