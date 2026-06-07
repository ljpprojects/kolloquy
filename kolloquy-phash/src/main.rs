pub mod hash;

use axum::{Json, extract::Path, response::Response};
use secrecy::{SecretBox, SecretSlice};
use serde::Deserialize;
use stack_string::SmallString;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};

pub const PASS2_PEPPER_SIZE: usize = 30;
pub const PASS2_THYME_SIZE: usize = 384;

pub struct State {
    thyme: SecretBox<[u8; PASS2_THYME_SIZE]>,
}

#[derive(Deserialize)]
struct PHashRequest {
    salt2: SmallString<40>, // Digest is in the url as not-padded Base64 url-safe encoded bytes
}

async fn handle_phash_req(Path(b64_digest): Path<SmallString<43>>, Json(body): Json<PHashRequest>) -> Response {
    let mut digest = [0u8; 32];
    BASE64_URL_SAFE_NO_PAD.encode_slice(b64_digest, &mut digest);

    // pass off
}

fn main() {

}