use kolloquy_consts::{KOLLOQUY_SERVICE_STR};
use zeroize::Zeroizing;

use crate::{PASS2_THYME_SIZE};

pub fn get_thyme_from_keyring(id: usize) -> Option<Zeroizing<[u8; PASS2_THYME_SIZE]>> {
    let entry = match keyring_core::Entry::new(KOLLOQUY_SERVICE_STR, &*format!("phash:thyme@{id}")) {
        Ok(e) => e,
        Err(e) => {
            #[cfg(feature = "logging")]
            tracing::error!(name: "Could not add thyme to keyring", ?e);

            return None
        }
    };

    let bytes = match entry.get_secret() {
        Ok(s) => s,
        Err(e) => {
            #[cfg(feature = "logging")]
            tracing::error!(name: "Could not get thyme from keyring", ?e);

            return None
        }
    };

    let mut secret = [0u8; PASS2_THYME_SIZE];

    if bytes.len() < PASS2_THYME_SIZE {
        return None
    }

    secret.copy_from_slice(&bytes);

    Some(Zeroizing::new(secret))
}

/// Puts the thyme to the native keyring
/// This should be run if USE_KEYRING is 1 and the `--set-thyme` flag is passed
pub fn put_thyme_to_keyring(thyme: &[u8; PASS2_THYME_SIZE], id: usize) -> Option<()> {
    let entry = match keyring_core::Entry::new(KOLLOQUY_SERVICE_STR, &*format!("phash:thyme@{id}")) {
        Ok(e) => e,
        Err(e) => {
            #[cfg(feature = "logging")]
            tracing::error!(name: "Could not add thyme to keyring", ?e);

            return None
        }
    };

    match entry.set_secret(thyme) {
        Ok(e) => Some(()),
        Err(e) => {
            #[cfg(feature = "logging")]
            tracing::error!(name: "Could not add thyme to keyring", ?e);

            None
        }
    }
}