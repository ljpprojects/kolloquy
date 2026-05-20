use alloc::{boxed::Box, string::{String, ToString}, vec};
use base64urlsafedata::Base64UrlSafeData;
use stack_string::SmallString;
use base64::{Engine, prelude::BASE64_STANDARD};
use webauthn_rs_proto::{COSEAlgorithm, Pu, PubKeyCredParams, PublicKeyCredentialCreationOptions, PublicKeyCredentialHints, RelyingParty, User as WebAuthnUser};

use crate::user::User;

pub const RP_NAME: &str = "Kolloquy";
pub const RP_ID: &str = "kolloquy.com";

pub fn generate_registration_options(user: &User) -> PublicKeyCredentialCreationOptions {
    let mut decoded_id = [0u8; 18];
    BASE64_STANDARD.decode_slice(user.id, &mut decoded_id).unwrap();

    let user_id = Base64UrlSafeData::new();
    user_id.extend_from_slice(&decoded_id);

    PublicKeyCredentialCreationOptions {
        rp: RelyingParty {
            name: RP_NAME.to_string(),
            id: RP_ID.to_string(),
        },
        user: WebAuthnUser {
            id: user_id,
            name: user.email.clone(),
            display_name: user.display_name.clone(),
        },
        pub_key_cred_params: vec![
            PubKeyCredParams {
                type_: "public-key".to_string(),
                alg: COSEAlgorithm::EDDSA as i64
            },
            PubKeyCredParams {
                type_: "public-key".to_string(),
                alg: COSEAlgorithm::ES256 as i64
            },
        ],
        timeout: Some(45000), // 45s
        authenticator_selection: None,
        attestation: None,
        attestation_formats: None,
        extensions: None,
        hints: Some(vec![PublicKeyCredentialHints::ClientDevice]),
    }
}