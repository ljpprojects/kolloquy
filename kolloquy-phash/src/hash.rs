use std::{env, sync::Arc};
use crate::{PASS2_PEPPER_SIZE, PASS2_PEPPER_VAR_NAME, PASS2_THYME_SIZE, ServerState};
use argon2::{Algorithm, Argon2, AssociatedData, KeyId, ParamsBuilder, PasswordHash, PasswordHasher, Version};
use base64::{Engine, engine::general_purpose::STANDARD};
use kolloquy_consts::{PASS1_DIGEST_SIZE, PASS2_DIGEST_SIZE, PASS2_MAGIC_DATA, PASS2_SALT_SIZE};
use secrecy::ExposeSecret;
use zeroize::Zeroizing;

pub fn compute_stage_2_digest(digest_1: [u8; PASS1_DIGEST_SIZE], salt_2: [u8; PASS2_SALT_SIZE], state: Arc<ServerState>) -> PasswordHash {
    let mut out = [0u8; PASS2_DIGEST_SIZE];

    let assoc_data = AssociatedData::new(&PASS2_MAGIC_DATA).unwrap();

    let alg = Algorithm::Argon2id;
    let ver = Version::V0x13;

    let params = ParamsBuilder::new() // We are only computing these like once ever for each user so it is fine to maybe go a bit overboard with parameters (like literally once ever hopefully, we can cache the hashes and nginx can serve those and the request hopefully never touches the server) but it might still be a bit hard on my poor 4GB-ram Pi 5...
        .data(assoc_data)
        .output_len(PASS2_DIGEST_SIZE)
        .m_cost(2u32.pow(17)) // 128MiB
        .t_cost(3)
        .p_cost(2)
        .build()
        .unwrap();

    let mut pepper = [0u8; PASS2_PEPPER_SIZE];
    STANDARD.decode_slice(env::var(PASS2_PEPPER_VAR_NAME).unwrap(), &mut pepper);

    let mut full_secret = Zeroizing::new([0u8; PASS2_PEPPER_SIZE + PASS2_THYME_SIZE]);
    (&mut full_secret[..PASS2_THYME_SIZE]).copy_from_slice(&**state.thyme);
    (&mut full_secret[PASS2_THYME_SIZE..]).copy_from_slice(&pepper);

    let ctx = Argon2::new_with_secret(&*full_secret, alg, ver, params).unwrap();

    ctx.hash_password_with_salt(&digest_1, &salt_2).unwrap()
}