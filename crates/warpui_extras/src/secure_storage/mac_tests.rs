use std::os::unix::fs::PermissionsExt;

use super::super::{Error, SecureStorage as _};
use super::SecureStorage;

fn storage(dir: &tempfile::TempDir) -> SecureStorage {
    // A service name no keychain holds, so the keychain fallback always misses.
    SecureStorage::new_with_path("dev.harp.secure-storage-test", dir.path().to_owned())
}

#[test]
fn values_round_trip_through_owner_only_files() {
    let dir = tempfile::tempdir().unwrap();
    let storage = storage(&dir);

    storage.write_value("token", "first").unwrap();
    storage.write_value("token", "second").unwrap();
    assert_eq!(storage.read_value("token").unwrap(), "second");

    let file = dir
        .path()
        .join("secrets/dev.harp.secure-storage-test-token");
    assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
    let secrets_dir = dir.path().join("secrets");
    assert_eq!(
        secrets_dir.metadata().unwrap().permissions().mode() & 0o777,
        0o700
    );

    storage.remove_value("token").unwrap();
    assert!(matches!(storage.read_value("token"), Err(Error::NotFound)));
}

#[test]
fn missing_values_report_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let storage = storage(&dir);

    assert!(matches!(storage.read_value("absent"), Err(Error::NotFound)));
    assert!(matches!(
        storage.remove_value("absent"),
        Err(Error::NotFound)
    ));
}

#[test]
#[ignore = "writes to the login keychain; run by hand with --ignored"]
fn keychain_values_move_to_files_on_first_read() {
    let dir = tempfile::tempdir().unwrap();
    let keychain = SecureStorage::new("dev.harp.secure-storage-test");
    let storage = storage(&dir);
    keychain.write_value("migrated", "from keychain").unwrap();

    assert_eq!(storage.read_value("migrated").unwrap(), "from keychain");
    assert!(matches!(
        keychain.read_value("migrated"),
        Err(Error::NotFound)
    ));
    assert_eq!(storage.read_value("migrated").unwrap(), "from keychain");

    storage.remove_value("migrated").unwrap();
}
