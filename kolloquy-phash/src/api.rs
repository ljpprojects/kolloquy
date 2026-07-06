use std::{fmt::Display, str::FromStr};

use argon2::PasswordHash;
use crate::consts::errcodes::{INVALID_DIGEST, INVALID_DIGEST_SIZE, INVALID_SALT, INVALID_SALT_SIZE, INVALID_VERSION, NO_VERSION_SPECIFIED, VERSION_MISMATCH};

pub enum PHashError<'a> {
    InvalidDigest(&'a str),
    InvalidDigestSize(&'a str),

    InvalidSalt(&'a str),
    InvalidSaltSize(&'a str),

    VersionMismatch(&'a str),
    VersionUnspecified(&'a str),
    InvalidVersion(&'a str),

    UnknownPanic,

    /// Different to Other in the way that an Unknown error did not have an error code, but an Other error did have an error code
    Unknown(&'a str),

    /// Different to Unknown in the way that an Other error had an error code, but an Unknown error did not have an error code
    Other(&'a str, &'a str)
}

impl Display for PHashError<'_> {
    // If you pass the result of this into res_body_to_result you should get an Err with the same PHashError
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDigest(message) => write!(f, "{INVALID_DIGEST}: {message}"),
            Self::InvalidDigestSize(message) => write!(f, "{INVALID_DIGEST_SIZE}: {message}"),

            Self::InvalidSalt(message) => write!(f, "{INVALID_SALT}: {message}"),
            Self::InvalidSaltSize(message) => write!(f, "{INVALID_SALT_SIZE}: {message}"),

            Self::VersionMismatch(message) => write!(f, "{VERSION_MISMATCH}: {message}"),
            Self::VersionUnspecified(message) => write!(f, "{NO_VERSION_SPECIFIED}: {message}"),
            Self::InvalidVersion(message) => write!(f, "{INVALID_VERSION}: {message}"),

            Self::UnknownPanic => write!(f, ""),
            Self::Unknown(body) => write!(f, "{body}"),
            Self::Other(errcode, message) => write!(f, "{errcode}: {message}"),
        }
    }
}

/// Given the response body, convert it to a Result<PasswordHash, PHashError>.
pub fn res_body_to_result<'a>(body: &'a str) -> Result<PasswordHash, PHashError<'a>> {
    if body.len() == 0 { // When the server panics an empty response is sent
        return Err(PHashError::UnknownPanic)
    }

     // An OK response will just be the PHC string, which doesn't contain colons, but errors are delimited by their code followed by a colon
    let Some((errcode, message)) = body.split_once(": ") else {
        return PasswordHash::new(&body).map_err(|_| PHashError::Unknown(body));
    };

    // We have an error
    // Which one?
    Err(match errcode {
        INVALID_DIGEST => PHashError::InvalidDigest(message),
        INVALID_DIGEST_SIZE => PHashError::InvalidDigestSize(message),
        INVALID_SALT => PHashError::InvalidDigest(message),
        INVALID_SALT_SIZE => PHashError::InvalidDigestSize(message),
        VERSION_MISMATCH => PHashError::VersionMismatch(message),
        NO_VERSION_SPECIFIED => PHashError::VersionUnspecified(message),
        INVALID_VERSION => PHashError::InvalidVersion(message),
        _ => PHashError::Other(errcode, message)
    })
}