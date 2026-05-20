use base64::engine::GeneralPurpose;

pub mod user;
pub mod chat;
pub mod query;

pub const B64_ENCODER: GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;