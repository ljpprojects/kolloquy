use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

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

/*
#[worker::send]
pub async fn new_session(
    Extension(state): Extension<WorkerState>,
    cookie: CookieManager,
) -> GenericAPIResponse<NewSessionResponse> {
    let session = get_session!(
        with
            cookie_jar: cookie,
            state: state
    );

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
*/