use alloc::{format, string::{String, ToString}, sync::Arc, vec::Vec};
use const_format::formatcp;
use http::{HeaderMap, HeaderValue, StatusCode, header::{self, CONTENT_TYPE}};
use wasm_bindgen::JsValue;
use worker::{crypto::DigestStreamAlgorithm, *};
use serde_json::json;
use chrono::{DateTime, Utc};

use crate::{crypto::{native_hmac_sha256, native_webcrypto_hash}, hex::hex_encode};

/// Call the AWS SES API and return the response
pub async fn send_ses_mail(
    env: Arc<Env>,
    from: &str,
    recipient: &str,
    subject: &str,
    content: &str,
) -> Result<Response> {
    let access_key = env.secret("AWS_ACCESS_KEY_ID").unwrap().to_string();
    let secret_key = env.secret("AWS_SECRET_ACCESS_KEY").unwrap().to_string();

    const REGION: &str = "ap-southeast-2";
    const SERVICE: &str = "ses";

    const HOST: &str = formatcp!("email.{REGION}.amazonaws.com");
    const URL: &str = formatcp!("https://{HOST}/v2/email/outbound-emails");

    let body = json!({
        "FromEmailAddress": from,
        "Destination": { "ToAddresses": [recipient] },
        "Content": {
            "Simple": {
                "Subject": { "Data": subject },
                "Body": { "Html": { "Data": content } }
            }
        }
    }).to_string();

    let now = DateTime::<Utc>::from_timestamp_millis(worker::Date::now().as_millis() as i64).unwrap();

    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string(); // ISO 8601 format
    let date_string = now.format("%Y%m%d").to_string(); // Does this even have an ISO?

    let body_hash = hex_encode(native_webcrypto_hash(DigestStreamAlgorithm::Sha256, &*body).await);

    let mut headers = HeaderMap::<HeaderValue>::with_capacity(4);

    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(header::HOST, HOST.parse().unwrap());
    headers.insert("x-amz-date", amz_date.parse().unwrap());
    headers.insert("x-amz-content-sha256", body_hash.parse().unwrap());

    let mut header_vec_sorted = headers.iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_string())).collect::<Vec<_>>();
    header_vec_sorted.sort_by_key(|(k, _)| k.clone());

    let sorted_header_keys = header_vec_sorted.iter().map(|(n, _)| n.to_string()).collect::<Vec<_>>();

    let canonical_req = format!(
        "POST\n/v2/email/outbound-emails\n\n{}\n\n{}\n{body_hash}",
        header_vec_sorted.iter().map(|(n, v)| format!("{n}:{}", v.trim())).collect::<Vec<_>>().join("\n"),
        sorted_header_keys.join(";"),
    );

    let canonical_req_hash = hex_encode(native_webcrypto_hash(DigestStreamAlgorithm::Sha256, canonical_req).await);

    let scope = format!(
        "{date_string}/{REGION}/ses/aws4_request"
    );

    // Why does AWS need to use such an AI-style, vague, general name as StringToSign???

    let string_that_shall_be_signed_in_due_time_and_is_named_in_unequivocally_unspecific_aws_fashion = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{canonical_req_hash}"
    );

    let date_key = native_hmac_sha256(
        format!("AWS4{secret_key}"),
        date_string,
    ).await;

    let date_region_key = native_hmac_sha256(
        date_key,
        REGION,
    ).await;

    let date_region_service_key = native_hmac_sha256(
        date_region_key,
        SERVICE,
    ).await;

    let signing_key = native_hmac_sha256(
        date_region_service_key,
        "aws4_request",
    ).await;

    let signature = native_hmac_sha256(signing_key, string_that_shall_be_signed_in_due_time_and_is_named_in_unequivocally_unspecific_aws_fashion).await;

    let authorisation = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope},SignedHeaders={},Signature={}",
        sorted_header_keys.join(";"),
        hex_encode(signature),
    );

    headers.insert("Authorization", authorisation.parse().unwrap());

    // Create a request
    let mut req_init = RequestInit::new();
    req_init
        .with_method(Method::Post)
        .with_headers(Headers::from(headers))
        .with_body(Some(JsValue::from_str(&*body)));

    let req = Request::new_with_init(URL, &req_init).unwrap();

    let mut res = worker::Fetch::Request(req)
        .send()
        .await?;

    if !StatusCode::from_u16(res.status_code()).unwrap().is_success() {
        return Err(worker::Error::RustError(format!("{:#?}", res.bytes().await.map(|b| String::from_utf8(b).unwrap()))))
    }

    Ok(res)
}
