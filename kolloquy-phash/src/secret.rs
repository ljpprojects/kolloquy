use kolloquy_consts::KOLLOQUY_SERVICE_STR;
use zeroize::Zeroizing;

use crate::PASS2_THYME_SIZE;

pub fn get_thyme_from_keyring(
    id: usize,
) -> Result<Zeroizing<[u8; PASS2_THYME_SIZE]>, keyring_core::Error> {
    let entry = match keyring_core::Entry::new(KOLLOQUY_SERVICE_STR, &*format!("phash:thyme.{id}"))
    {
        Ok(e) => e,
        Err(e) => {
            #[cfg(feature = "tracing")]
            tracing::error!(name: "Could not add thyme to keyring", ?e);

            return Err(e);
        }
    };

    let bytes = match entry.get_secret() {
        Ok(s) => s,
        Err(e) => {
            #[cfg(feature = "tracing")]
            tracing::error!(name: "Could not get thyme from keyring", ?e);

            return Err(e);
        }
    };

    let mut secret = [0u8; PASS2_THYME_SIZE];

    if bytes.len() < PASS2_THYME_SIZE {
        panic!("The thyme in the keyring isn't large enough????");
    }

    secret.copy_from_slice(&bytes);

    Ok(Zeroizing::new(secret))
}

/// Puts the thyme to the native keyring
/// This should be run if USE_KEYRING is 1 and the `--set-thyme` flag is passed
pub fn put_thyme_to_keyring(
    thyme: &[u8; PASS2_THYME_SIZE],
    id: usize,
) -> Result<(), keyring_core::Error> {
    let entry = match keyring_core::Entry::new(KOLLOQUY_SERVICE_STR, &*format!("phash:thyme.{id}"))
    {
        Ok(e) => e,
        Err(e) => {
            #[cfg(feature = "tracing")]
            tracing::error!(name: "Could not add thyme to keyring", ?e);

            return Err(e);
        }
    };

    match entry.set_secret(thyme) {
        Ok(_) => Ok(()),
        Err(e) => {
            #[cfg(feature = "tracing")]
            tracing::error!(name: "Could not add thyme to keyring", ?e);

            Err(e)
        }
    }
}
