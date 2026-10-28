use super::Store;

const PANE: &str = "00b9567f71314a049aee79a811004b44";
const SESSION: &str = "61de7da2-2ffd-4560-b067-90c140d77eba";

fn store_with_session() -> Store {
    let mut store = Store::default();
    store.record(PANE.to_owned(), Some(SESSION));
    store
}

#[test]
fn save_before_resumed_claude_reports_keeps_the_session() {
    let mut store = store_with_session();
    assert_eq!(
        store.take_for_resume(PANE.to_owned()).as_deref(),
        Some(SESSION)
    );

    assert!(!store.record(PANE.to_owned(), None));

    assert_eq!(store.sessions.get(PANE).map(String::as_str), Some(SESSION));
}

#[test]
fn exiting_claude_after_resume_reported_forgets_the_session() {
    let mut store = store_with_session();
    store.take_for_resume(PANE.to_owned());
    store.record(PANE.to_owned(), Some(SESSION));

    assert!(store.record(PANE.to_owned(), None));

    assert!(!store.sessions.contains_key(PANE));
}

#[test]
fn exiting_claude_in_a_pane_never_resumed_forgets_the_session() {
    let mut store = store_with_session();

    assert!(store.record(PANE.to_owned(), None));

    assert!(!store.sessions.contains_key(PANE));
}

#[test]
fn resumed_pane_reporting_a_new_session_replaces_the_old_one() {
    let mut store = store_with_session();
    store.take_for_resume(PANE.to_owned());

    assert!(store.record(
        PANE.to_owned(),
        Some("c90a34e6-dc8b-4f44-afdf-aaed71270f22")
    ));

    assert_eq!(
        store.sessions.get(PANE).map(String::as_str),
        Some("c90a34e6-dc8b-4f44-afdf-aaed71270f22")
    );
}

#[test]
fn take_for_resume_rejects_ids_unsafe_for_the_shell() {
    let mut store = Store::default();
    store.record(PANE.to_owned(), Some("abc; rm -rf ~"));

    assert_eq!(store.take_for_resume(PANE.to_owned()), None);
    assert!(store.record(PANE.to_owned(), None));
}
