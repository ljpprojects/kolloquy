use core::time::Duration;

use alloc::{format, string::{String, ToString}, sync::Arc, vec::Vec};
use axum::{Extension, routing::post};
use axum_cookie::{CookieLayer, CookieManager, cookie::Cookie, prelude::SameSite};
use base64::{Engine, prelude::BASE64_STANDARD};
use handlebars::Handlebars;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use worker::{Context, Env};

use crate::{api::GenericAPIResponse, crypto::random_bytes_generic, d1::D1Interface, kv::KvInterface, realtime::RealtimeSession, session::Session, state::WorkerState, user::{User, UserIdentifyingKey}};

/***** /rt/session/new *****/

#[derive(Deserialize)]
pub struct CfNewSessionResponse {
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,

    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,

    #[serde(rename = "errorDescription")]
    pub error_description: Option<String>,
}

#[repr(u32)]
#[derive(Serialize)]
pub enum NewSessionErrorCode {
    Other,
    Unauthenticated,
    NoSuchUser,
}

#[derive(Serialize)]
pub struct NewSessionError {
    pub code: NewSessionErrorCode,
    pub message: String,
}

#[derive(Serialize)]
pub struct NewSessionResponse {
    pub session_id: Option<String>,
    pub error: Option<NewSessionError>,
}

/***** /rt/session/datachannels/establish *****/

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct CfDataTransportChannel {
    pub location: String,
    pub dataChannelName: String, // should always be server-events
    pub id: i32,
}

#[derive(Deserialize)]
struct CfDataTransportSessionDesc {
    pub sdp: String,
    pub r#type: String,
}

#[derive(Deserialize)]
pub struct CfDataTransportResponse {
    #[serde(rename = "requiresImmediateRenegotiation")]
    pub requires_immediate_renegotiation: bool,
    pub datachannel: CfDataTransportChannel,

    #[serde(rename = "sessionDescription")]
    pub session_description: CfDataTransportSessionDesc,

    #[serde(rename = "errorCode")]
    pub error_code: Option<i32>,

    #[serde(rename = "errorDescription")]
    pub error_description: Option<String>,
}

#[derive(Deserialize)]
pub struct EstablishDataTransportRequest {
    pub sdp_offer: String,
}

/***** /rt/session/renegotiate *****/

#[derive(Serialize, Deserialize)]
pub struct RenegotiateResponse {
    #[serde(rename = "errorCode")]
    pub error_code: Option<i32>,

    #[serde(rename = "errorDescription")]
    pub error_description: Option<String>,
}

#[derive(Deserialize)]
pub struct RenegotiateSessionRequest {
    pub sdp_offer: String,
}

/***** /rt/session/datachannels/new *****/

#[derive(Serialize, Deserialize)]
pub struct DataChanAddChan {
    #[serde(rename = "dataChannelName")]
    pub data_channel_name: String,
    pub location: String,
    pub id: i32,
}

#[derive(Deserialize)]
pub struct DataChanAddResp {
    pub datachannels: Vec<DataChanAddChan>,

    #[serde(rename = "errorCode")]
    pub error_code: Option<i32>,

    #[serde(rename = "errorDescription")]
    pub error_description: Option<String>,
}

/***** HANDLERS ******/

#[worker::send]
pub async fn new_session(
    Extension(state): Extension<WorkerState>,
    cookie: CookieManager,
) -> GenericAPIResponse<NewSessionResponse> {
    let session = {
        match (cookie.get("ssid"), cookie.get("rftk")) {
            (Some(ssid), _) => {
                let decoded_ssid: [u8; 18] = BASE64_STANDARD.decode(ssid.value()).unwrap().try_into().unwrap();

                // Ensure that a session exists for the user
                match Session::fetch_from_remote(&decoded_ssid, state.env.clone()).await {
                    Err(e) => {
                        return GenericAPIResponse(
                            StatusCode::BAD_GATEWAY,
                            &[("Content-Type", "application.json")],
                            NewSessionResponse {
                                session_id: None,
                                error: Some(NewSessionError {
                                    code: NewSessionErrorCode::Other,
                                    message: format!("502 Bad Gateway (error fetching from KV): {e}"),
                                })
                            }
                        )
                    },
                    Ok(Some(session)) => session,
                    Ok(None) => return GenericAPIResponse(
                        StatusCode::TEMPORARY_REDIRECT,
                        &[("Content-Type", "application/json")],
                        NewSessionResponse {
                            session_id: None,
                            error: Some(NewSessionError {
                                code: NewSessionErrorCode::Unauthenticated,
                                message: "Unauthorised (did you call /auth/login?)".to_string(),
                            })
                        }
                    )
                }
            },
            (None, Some(rftk)) => {
                let decoded_rftk: [u8; 32] = BASE64_STANDARD.decode(rftk.value()).unwrap().try_into().unwrap();
                let user_id: [u8; 18] = decoded_rftk[14..].try_into().unwrap();

                match Session::fetch_from_remote(&user_id, state.env.clone()).await {
                    Ok(Some(session)) => {
                        if let Err(e) = session.put_to_remote(state.env.clone()).await {
                            return GenericAPIResponse(
                                StatusCode::BAD_GATEWAY,
                                &[("Content-Type", "application/json")],
                                NewSessionResponse {
                                    session_id: None,
                                    error: Some(NewSessionError {
                                        code: NewSessionErrorCode::Other,
                                        message: format!("502 Bad Gateway (error putting to KV): {e}")
                                    })
                                }
                            )
                        }

                        let session_cookie = Cookie::new("ssid", BASE64_STANDARD.encode(session.session_id()))
                            .with_secure(true)
                            .with_http_only(true)
                            .with_max_age(Duration::from_secs(30 * 60))
                            .with_same_site(SameSite::Strict)
                            .with_path("/");

                        // Set the session token cookie
                        cookie.add(session_cookie);

                        // Create a refresh token (112 random bits + 144 bit user id)
                        let refresh_token: [u8; 32] =
                            [random_bytes_generic::<14>().as_slice(), &*BASE64_STANDARD.decode(user_id).unwrap()]
                                .concat()
                                .try_into()
                                .unwrap();

                        let refresh_token_cookie = Cookie::new("rftk", BASE64_STANDARD.encode(refresh_token))
                            .with_secure(true)
                            .with_http_only(true)
                            .with_max_age(Duration::from_secs(45 * 24 * 60 * 60))
                            .with_same_site(SameSite::Strict)
                            .with_path("/");

                        // Rotate the refresh token cookie
                        cookie.add(refresh_token_cookie);

                        session
                    },
                    Ok(None) => return GenericAPIResponse(
                        StatusCode::TEMPORARY_REDIRECT,
                        &[("Content-Type", "application/json")],
                        NewSessionResponse {
                            session_id: None,
                            error: Some(NewSessionError {
                                code: NewSessionErrorCode::NoSuchUser,
                                message: "No such user (did you call /auth/register?)".to_string()
                            })
                        }
                    ),
                    Err(e) => return GenericAPIResponse(
                        StatusCode::BAD_GATEWAY,
                        &[("Content-Type", "application/json")],
                        NewSessionResponse {
                            session_id: None,
                            error: Some(NewSessionError {
                                code: NewSessionErrorCode::Other,
                                message: format!("502 Bad Gateway (error putting to KV): {e}")
                            })
                        }
                    )
                }
            },
            (None, None) => return GenericAPIResponse(
                StatusCode::TEMPORARY_REDIRECT,
                &[("Content-Type", "application/json")],
                NewSessionResponse {
                    session_id: None,
                    error: Some(NewSessionError {
                        code: NewSessionErrorCode::NoSuchUser,
                        message: "Unauthorised (did you call /auth/login?)".to_string(),
                    })
                }
            )
        }
    };

    let user = match User::fetch_from_remote(&UserIdentifyingKey::SessionId(*session.session_id()), state.env.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => return GenericAPIResponse(
            StatusCode::TEMPORARY_REDIRECT,
            &[("Content-Type", "application/json")],
            NewSessionResponse {
                session_id: None,
                error: Some(NewSessionError {
                    code: NewSessionErrorCode::NoSuchUser,
                    message: "Unauthorised (did you call /auth/login?)".to_string(),
                })
            }
        ),
        Err(e) => return GenericAPIResponse(
            StatusCode::BAD_GATEWAY,
            &[("Content-Type", "application/json")],
            NewSessionResponse {
                session_id: None,
                error: Some(NewSessionError {
                    code: NewSessionErrorCode::Other,
                    message: format!("502 Bad Gateway (error retrieving user): {e}")
                })
            }
        ),
    };

    let rt_session = match RealtimeSession::new(user.id, state.env.clone()).await {
        Ok(s) => s,
        Err(e) => return GenericAPIResponse(
            StatusCode::BAD_GATEWAY,
            &[("Content-Type", "application/json")],
            NewSessionResponse {
                session_id: None,
                error: Some(NewSessionError {
                    code: NewSessionErrorCode::Other,
                    message: format!("502 Bad Gateway (error creating realtime session): {e}")
                })
            }
        ),
    };

    GenericAPIResponse(
        StatusCode::CREATED,
        &[("Content-Type", "application/json")],
        NewSessionResponse {
            session_id: Some(rt_session.session_id),
            error: None,
        },
    )
}

/***** ROUTER (should be nested under /rt) *****/

pub fn realtime_router(env: Arc<Env>, ctx: Arc<Context>, hbars: Arc<Handlebars<'static>>) -> axum::Router {
    axum::Router::new()
        .route("/session/new", post(new_session))
        .layer(CookieLayer::default())
        .layer(Extension(WorkerState {
            env,
            ctx,
            hbars,
        }))
}
