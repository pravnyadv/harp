use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use security_framework::os::macos::keychain::SecKeychain;
use security_framework::os::macos::keychain_item::SecKeychainItem;
use security_framework::os::macos::passwords::SecKeychainItemPassword;

use super::{Error, SecureStorage as _};

pub struct SecureStorage {
    service_name: String,
    /// Holds one owner-only file per key instead of the login keychain. A self-signed build has
    /// no team ID, so macOS partitions keychain items by the build's cdhash and asks for access
    /// again after every update.
    storage_dir: Option<PathBuf>,
}

impl SecureStorage {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_owned(),
            storage_dir: None,
        }
    }

    /// Stores values under `secrets` in `storage_dir`, or in the keychain when it is empty.
    pub fn new_with_path(service_name: &str, storage_dir: PathBuf) -> Self {
        Self {
            service_name: service_name.to_owned(),
            storage_dir: (!storage_dir.as_os_str().is_empty()).then(|| storage_dir.join("secrets")),
        }
    }
}

impl super::SecureStorage for SecureStorage {
    fn write_value(&self, key: &str, value: &str) -> Result<(), Error> {
        match &self.storage_dir {
            Some(dir) => write_owner_only(dir, &self.file_name(key), value),
            None => self.write_keychain_value(key, value),
        }
    }

    fn read_value(&self, key: &str) -> Result<String, Error> {
        let Some(dir) = &self.storage_dir else {
            return self.read_keychain_value(key);
        };
        match fs::read_to_string(dir.join(self.file_name(key))) {
            Ok(value) => Ok(value),
            Err(err) if err.kind() == io::ErrorKind::NotFound => self.migrate_from_keychain(key),
            Err(err) => Err(Error::Unknown(err.into())),
        }
    }

    fn remove_value(&self, key: &str) -> Result<(), Error> {
        let Some(dir) = &self.storage_dir else {
            return self.remove_keychain_value(key);
        };
        let removed_file = match fs::remove_file(dir.join(self.file_name(key))) {
            Ok(()) => true,
            Err(err) if err.kind() == io::ErrorKind::NotFound => false,
            Err(err) => return Err(Error::Unknown(err.into())),
        };
        // A keychain copy left behind would be migrated back on the next read.
        match self.remove_keychain_value(key) {
            Err(Error::NotFound) if removed_file => Ok(()),
            result => result,
        }
    }
}

impl SecureStorage {
    fn file_name(&self, key: &str) -> String {
        format!("{}-{key}", self.service_name)
    }

    /// Moves a value an earlier build stored in the keychain into the storage dir.
    fn migrate_from_keychain(&self, key: &str) -> Result<String, Error> {
        let value = self.read_keychain_value(key)?;
        self.write_value(key, &value)?;
        if let Err(err) = self.remove_keychain_value(key) {
            log::warn!("Could not remove migrated keychain item {key}: {err}");
        }
        Ok(value)
    }

    fn write_keychain_value(&self, key: &str, value: &str) -> Result<(), Error> {
        SecKeychain::default()?
            .set_generic_password(&self.service_name, key, value.as_bytes())
            .map_err(Into::into)
    }

    fn read_keychain_value(&self, key: &str) -> Result<String, Error> {
        let (password, _) = self.get_password_item(key)?;
        String::from_utf8(password.as_ref().to_vec())
            .map_err(|err| Error::DecodeError(err.utf8_error()))
    }

    fn remove_keychain_value(&self, key: &str) -> Result<(), Error> {
        let (_, item) = self.get_password_item(key)?;
        item.delete();
        Ok(())
    }

    fn get_password_item(
        &self,
        key: &str,
    ) -> Result<(SecKeychainItemPassword, SecKeychainItem), Error> {
        let keychain = SecKeychain::default()?;
        keychain
            .find_generic_password(&self.service_name, key)
            .map_err(|_| Error::NotFound)
    }
}

/// Writes through a temporary file so a crash never leaves a truncated secret, with the
/// directory at 0700 and the file at 0600.
fn write_owner_only(dir: &Path, file_name: &str, value: &str) -> Result<(), Error> {
    let write = || -> io::Result<()> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        let tmp = dir.join(format!(".{file_name}.tmp"));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(value.as_bytes())?;
        file.sync_all()?;
        fs::rename(tmp, dir.join(file_name))
    };
    write().map_err(|err| Error::Unknown(anyhow!(err)))
}

impl From<security_framework::base::Error> for Error {
    fn from(value: security_framework::base::Error) -> Self {
        Error::Unknown(anyhow!(value))
    }
}

#[cfg(test)]
#[path = "mac_tests.rs"]
mod tests;
