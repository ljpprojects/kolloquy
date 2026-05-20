use alloc::{borrow::ToOwned, format, string::{String, ToString}, sync::Arc, vec::Vec};
use stack_string::SmallString;
use stackvec::StackVec;
use url::Url;
use wasm_bindgen::JsValue;
use worker::{Env, crypto::{DigestStream, DigestStreamAlgorithm}};

use crate::crypto::native_webcrypto_hash;

async fn call_api(hash_chars: &str) -> Result<Vec<(String, u32)>, worker::Error> {
    let formatted_url = format!("https://api.pwnedpasswords.com/range/{hash_chars}");

    let req = worker::Fetch::Url(Url::parse(&*formatted_url).unwrap());
    let mut res = req.send().await?;
    let res_body = String::from_utf8(res.bytes().await?).unwrap();

    Ok(res_body.lines().map(|s| {
        let [hash, count_str] = s.split(':').collect::<StackVec<[&str; 2]>>()[..] else {
            unreachable!()
        };

        (hash.to_string(), u32::from_str_radix(count_str, 10).unwrap())
    }).collect())
}

/// **HASH SHOULD BE IN ALL CAPS**
pub async fn password_hash_pwned(hash: &str) -> Result<bool, worker::Error> {
    // Oh the hidden allocations!

    // Call the api and discard counts
    let hash_suffixes: Vec<String> = call_api(&hash[0..5]).await?.iter().map(|(h, _)| h.clone()).collect();

    // Check if the hash is contained in the result
    // We can use binary search because the API gives sorted results
    // We need not append back the first 5 characters when searching
    Ok(hash_suffixes.binary_search(&hash[5..].to_owned()).is_ok())
}

pub async fn password_plaintext_pwned(plaintext: &str) -> Result<bool, worker::Error> {
    // Hash the password
    let bytes: [u8; 20] = native_webcrypto_hash(DigestStreamAlgorithm::Sha1, plaintext).await.unwrap();
    let mut hex_hash = SmallString::<40>::new();
    hex::encode_to_slice(
        bytes,
        unsafe { hex_hash.as_bytes_mut() }, // Aliasing
    ).unwrap();
    
    unsafe {
        for b in hex_hash.as_bytes_mut() {
            *b = b.to_ascii_uppercase();
        }
    }

    password_hash_pwned(hex_hash.as_str()).await
}
