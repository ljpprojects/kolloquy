pub mod phash;

#[cfg(all(feature = "no-phash-serv", feature = "use-phash-serv"))]
#[allow(non_upper_case_globals)]
const _feat_chk: () = {
    compile_error!(
        "The no-phash-serv and use-phash-serv features are mutually"
        + "exclusive. Enable one only."
    )
};

pub fn random_bytes_pub<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    openssl::rand::rand_bytes(&mut bytes).unwrap();

    bytes
}

pub fn random_bytes_priv<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    openssl::rand::rand_priv_bytes(&mut bytes).unwrap();

    bytes
}

use kolloquy_consts::{TOTP_WRAP_KEY_NAME, PASS1_DIGEST_SIZE};
use openssl::aes::AesKey;

pub fn unwrap_totp_key(wrapped_secret: [u8; 28]) -> [u8; 20] {
    let mut wrap_key_bytes = [0u8; 32];

    // ErrorHandling? I... I don't know that spell.
    let encoded_key_bytes: String = std::env::var(TOTP_WRAP_KEY_NAME).unwrap();
    kolloquy_consts::base64_decode_slice(
        encoded_key_bytes.as_bytes(),
        &mut wrap_key_bytes,
        kolloquy_consts::base64url_rev_alphabet(),
    );

    let wrap_key = AesKey::new_decrypt(&wrap_key_bytes).unwrap();

    let mut unwrap_bytes = [0u8; 20];
    openssl::aes::unwrap_key(&wrap_key, None, &mut unwrap_bytes, &wrapped_secret);

    unwrap_bytes
}

pub fn wrap_totp_key(secret: [u8; 20]) -> [u8; 28] {
    let mut wrap_key_bytes = [0u8; 32];

    // ErrorHandling? I... I don't know that spell.
    let encoded_key_bytes: String = std::env::var(TOTP_WRAP_KEY_NAME).unwrap();
    kolloquy_consts::base64_decode_slice(
        encoded_key_bytes.as_bytes(),
        &mut wrap_key_bytes,
        kolloquy_consts::base64url_rev_alphabet(),
    );

    let wrap_key = AesKey::new_encrypt(&wrap_key_bytes).unwrap();

    let mut wrap_bytes = [0u8; 28];
    openssl::aes::wrap_key(&wrap_key, None, &mut wrap_bytes, &secret);

    wrap_bytes
}
