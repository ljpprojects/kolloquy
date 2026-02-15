use alloc::string::{String, ToString};
use core::str::FromStr;
use axum::{body::Body, response::{IntoResponse, Response}};
use http::{HeaderName, HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};

pub struct GenericAPIResponse<T: Serialize>(pub StatusCode, pub &'static [(&'static str, &'static str)], pub T);

impl<T: Serialize> From<(StatusCode, &'static [(&'static str, &'static str)], T)> for GenericAPIResponse<T> {
    fn from(value: (StatusCode, &'static [(&'static str, &'static str)], T)) -> Self {
        Self(value.0, value.1, value.2)
    }
}

impl<T: Serialize> IntoResponse for GenericAPIResponse<T> {
    fn into_response(self) -> Response {
        let mut resp = Response::builder()
            .status(self.0)
            .body(Body::new(serde_json::to_string(&self.2).unwrap()))
            .unwrap();

        let headers = resp.headers_mut();

        headers.extend(
            self.1
                .into_iter()
                .map(|(n, v)| (HeaderName::from_str(&*n.to_ascii_lowercase()).unwrap(), HeaderValue::from_static(v)))
        );

        resp
    }
}

pub struct GenericResponse<T: ToString>(pub StatusCode, pub &'static [(&'static str, &'static str)], pub T);

impl<T: ToString> From<(StatusCode, &'static [(&'static str, &'static str)], T)> for GenericResponse<T> {
    fn from(value: (StatusCode, &'static [(&'static str, &'static str)], T)) -> Self {
        Self(value.0, value.1, value.2)
    }
}

impl<T: ToString> IntoResponse for GenericResponse<T> {
    fn into_response(self) -> Response {
        let mut resp = Response::builder()
            .status(self.0)
            .body(Body::new(self.2.to_string()))
            .unwrap();

        let headers = resp.headers_mut();

        headers.extend(
            self.1
                .into_iter()
                .map(|(n, v)| (HeaderName::from_str(&*n.to_ascii_lowercase()).unwrap(), HeaderValue::from_static(v)))
        );

        resp
    }
}

/*********** /auth/register ***********/

#[repr(u32)]
#[derive(Serialize, Deserialize)]
pub enum AuthRegisterErrorCode {
    Other,
    MalformedEmail,
    MalformedDisplayName,
    MalformedPassword,
    UserExists,
    PasswordPwned,
}

#[derive(Serialize, Deserialize)]
pub struct AuthRegisterError {
    pub code: AuthRegisterErrorCode,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthRegisterResponse {
    pub status: String,
    pub error: Option<AuthRegisterError>,
}

#[derive(Deserialize)]
pub struct AuthRegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
}

/*********** /auth/verify ***********/

#[repr(u32)]
#[derive(Serialize, Deserialize)]
pub enum AuthVerifyErrorCode {
    Other,
    MalformedCode,
    IncorrectCode,
    VerificationAborted,
    Unauthenticated,
    RateLimit,
}

#[derive(Serialize, Deserialize)]
pub struct AuthVerifyError {
    pub code: AuthVerifyErrorCode,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthVerifyResponse {
    pub status: String,
    pub error: Option<AuthVerifyError>,
}

#[derive(Deserialize)]
pub struct AuthVerifyRequest {
    pub code: String,
}

/*********** /auth/login ***********/

#[repr(u32)]
#[derive(Serialize, Deserialize)]
pub enum AuthLoginErrorCode {
    Other,
    MalformedEmail,
    // No such user exists with the credentials the client gave
    // This could be either that no user with the email exists or that user's password is wrong
    NoSuchUser,
}

#[derive(Serialize, Deserialize)]
pub struct AuthLoginError {
    pub code: AuthLoginErrorCode,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthLoginResponse {
    pub status: String,
    pub error: Option<AuthLoginError>,
}

#[derive(Deserialize)]
pub struct AuthLoginRequest {
    pub email: String,
    pub password: String,
}
