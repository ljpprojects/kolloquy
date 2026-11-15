use std::fmt::Display;

use argon2::PasswordHash;
use crate::consts::errcodes::{INVALID_DIGEST, INVALID_DIGEST_SIZE, INVALID_SALT, INVALID_SALT_SIZE, INVALID_VERSION, NO_VERSION_SPECIFIED, VERSION_MISMATCH};

pub enum PHashError {
    InvalidDigest(Box<str>),
    InvalidDigestSize(Box<str>),

    InvalidSalt(Box<str>),
    InvalidSaltSize(Box<str>),

    VersionMismatch(Box<str>),
    VersionUnspecified(Box<str>),
    InvalidVersion(Box<str>),

    UnknownPanic,

    /// Different to Other in the way that an Unknown error did not have an error code, but an Other error did have an error code
    Unknown(Box<str>),

    /// Different to Unknown in the way that an Other error had an error code, but an Unknown error did not have an error code
    Other(Box<str>, Box<str>)
}

impl Display for PHashError {
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
pub fn res_body_to_result(body: &str) -> Result<PasswordHash, PHashError> {
    if body.len() == 0 { // When the server panics an empty response is sent
        return Err(PHashError::UnknownPanic)
    }

     // An OK response will just be the PHC string, which doesn't contain colons, but errors are delimited by their code followed by a colon
    let Some((errcode, message)) = body.split_once(": ") else {
        return PasswordHash::new(&body).map_err(|_| PHashError::Unknown(Box::from(body)));
    };

    // We have an error
    // Which one?
    Err(match errcode {
        INVALID_DIGEST => PHashError::InvalidDigest(Box::from(message)),
        INVALID_DIGEST_SIZE => PHashError::InvalidDigestSize(Box::from(message)),
        INVALID_SALT => PHashError::InvalidDigest(Box::from(message)),
        INVALID_SALT_SIZE => PHashError::InvalidDigestSize(Box::from(message)),
        VERSION_MISMATCH => PHashError::VersionMismatch(Box::from(message)),
        NO_VERSION_SPECIFIED => PHashError::VersionUnspecified(Box::from(message)),
        INVALID_VERSION => PHashError::InvalidVersion(Box::from(message)),
        _ => PHashError::Other(Box::from(errcode), Box::from(message))
    })
}