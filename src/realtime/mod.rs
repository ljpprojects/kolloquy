//! This moudle abstracts away Cloudflare's Call & TURN API

pub mod turn;
pub mod api;

use alloc::{borrow::ToOwned, format, string::{String, ToString}, sync::Arc, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::JsValue;
use worker::{Env, KvError, Method, Request, RequestInit};

use crate::{kv::KvInterface, realtime::api::{CfNewSessionResponse, DataChanAddResp, CfDataTransportResponse, RenegotiateResponse}};

pub const RT_API_BASE: &str = "https://rtc.live.cloudflare.com/v1";

#[derive(Serialize, Deserialize, Clone)]
pub struct RealtimeSessionData {
    session_id: String,

    /// The names of all data tracks local to the user associated with them
    #[serde(skip_serializing_if = "Option::is_none")]
    data_tracks: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct RealtimeSession {
    user_id: String,
    session_id: String,
    session_data_tracks: Vec<String>,
}

pub const KV_CACHE_TTL: u64 = 30 * 60;
pub const RT_SESSION_MAX_AGE: u64 = 24 * 60 * 60;

impl RealtimeSession {
    pub async fn new(user_id: String, env: Arc<Env>) -> worker::Result<Self> {
        let cf_sfu_app_id = env.secret("CF_SFU_APP_ID").unwrap().to_string();
        let cf_sfu_api_tk = env.secret("CF_SFU_API_TK").unwrap().to_string();

        let headers = worker::Headers::new();

        headers.set("Authorization", &*cf_sfu_api_tk).unwrap();
        headers.set("Content-Type", "application/json").unwrap();

        let mut req_init = RequestInit::new();
        req_init
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(None);

        let request = Request::new_with_init(
            &*format!("{RT_API_BASE}/apps/{cf_sfu_app_id}/sessions/new"),
            &req_init
        ).unwrap();

        let mut res = worker::Fetch::Request(request).send().await?;

        let resp = res.json::<CfNewSessionResponse>().await?;

        Ok(Self {
            user_id,
            session_id: resp.session_id.unwrap(),
            session_data_tracks: Vec::default(),
        })
    }

    pub async fn renegotiate(&self, sdp_answer: String, env: Arc<Env>) -> worker::Result<RenegotiateResponse> {
        let cf_sfu_app_id = env.secret("CF_SFU_APP_ID").unwrap().to_string();
        let cf_sfu_api_tk = env.secret("CF_SFU_API_TK").unwrap().to_string();

        let headers = worker::Headers::new();

        headers.set("Authorization", &*cf_sfu_api_tk).unwrap();
        headers.set("Content-Type", "application/json").unwrap();

        let body = json!({
            "sessionDescription": {
                "type": "answer",
                "sdp": sdp_answer
            }
        });

        let mut req_init = RequestInit::new();
        req_init
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(JsValue::from_str(&*body.to_string())));

        let request = Request::new_with_init(
            &*format!(
                "{RT_API_BASE}/apps/{cf_sfu_app_id}/sessions/{}/renegotiate",
                self.session_id
            ),
            &req_init
        ).unwrap();

        let mut res = worker::Fetch::Request(request).send().await?;

        res.json::<RenegotiateResponse>().await
    }

    pub async fn establish_data_chan_transport_sdp(&mut self, sdp: String, env: Arc<Env>) -> worker::Result<CfDataTransportResponse> {
        let cf_sfu_app_id = env.secret("CF_SFU_APP_ID").unwrap().to_string();
        let cf_sfu_api_tk = env.secret("CF_SFU_API_TK").unwrap().to_string();

        let headers = worker::Headers::new();

        headers.set("Authorization", &*cf_sfu_api_tk).unwrap();
        headers.set("Content-Type", "application/json").unwrap();

        let body = json!({
            "dataChannel": {
                "location": "remote",
                "dataChannelName": "server-events",
            },
            "sessionDescription": { "type": "offer", "sdp": sdp }
        });

        let mut req_init = RequestInit::new();
        req_init
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(JsValue::from_str(&*body.to_string())));

        let request = Request::new_with_init(
            &*format!(
                "{RT_API_BASE}/apps/{cf_sfu_app_id}/sessions/{}/datachannels/establish",
                self.session_id
            ),
            &req_init
        ).unwrap();

        let mut res = worker::Fetch::Request(request).send().await?;

        res.json::<CfDataTransportResponse>().await
    }

    pub async fn new_data_chan_local(&mut self, name: String, env: Arc<Env>) -> worker::Result<DataChanAddResp> {
        let cf_sfu_app_id = env.secret("CF_SFU_APP_ID").unwrap().to_string();
        let cf_sfu_api_tk = env.secret("CF_SFU_API_TK").unwrap().to_string();

        let headers = worker::Headers::new();

        headers.set("Authorization", &*cf_sfu_api_tk).unwrap();
        headers.set("Content-Type", "application/json").unwrap();

        let body = json!({
            "dataChannel": {
                "location": "local",
                "dataChannelName": name,
            },
        });

        let mut req_init = RequestInit::new();
        req_init
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(JsValue::from_str(&*body.to_string())));

        let request = Request::new_with_init(
            &*format!(
                "{RT_API_BASE}/apps/{cf_sfu_app_id}/sessions/{}/datachannels/new",
                self.session_id
            ),
            &req_init
        ).unwrap();

        let mut res = worker::Fetch::Request(request).send().await?;

        let res = res.json::<DataChanAddResp>().await?;

        self.session_data_tracks.extend(res.datachannels.iter().map(|c| c.data_channel_name.clone()));

        Ok(res)
    }

    pub async fn new_data_chan_remote(&mut self, name: String, other_session_id: String, env: Arc<Env>) -> worker::Result<DataChanAddResp> {
        let cf_sfu_app_id = env.secret("CF_SFU_APP_ID").unwrap().to_string();
        let cf_sfu_api_tk = env.secret("CF_SFU_API_TK").unwrap().to_string();

        let headers = worker::Headers::new();

        headers.set("Authorization", &*cf_sfu_api_tk).unwrap();
        headers.set("Content-Type", "application/json").unwrap();

        let body = json!({
            "dataChannel": {
                "location": "remote",
                "sessionId": other_session_id,
                "dataChannelName": name,
            },
        });

        let mut req_init = RequestInit::new();
        req_init
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(JsValue::from_str(&*body.to_string())));

        let request = Request::new_with_init(
            &*format!(
                "{RT_API_BASE}/apps/{cf_sfu_app_id}/sessions/{}/datachannels/new",
                self.session_id
            ),
            &req_init
        ).unwrap();

        let mut res = worker::Fetch::Request(request).send().await?;

        let res = res.json::<DataChanAddResp>().await?;

        self.session_data_tracks.extend(res.datachannels.iter().map(|c| c.data_channel_name.clone()));

        Ok(res)
    }
}

impl KvInterface for RealtimeSession {
    type Key = str;

    async fn fetch_from_remote(user_id: &str, env: Arc<Env>) -> Result<Option<Self>, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let Some(data) = kv.get(&*format!("rt:{user_id}"))
            .cache_ttl(KV_CACHE_TTL)
            .json::<RealtimeSessionData>()
            .await?
        else {
            return Ok(None)
        };

        Ok(Some(Self {
            user_id: user_id.to_owned(),
            session_id: data.session_id,
            session_data_tracks: data.data_tracks.unwrap_or_default()
        }))
    }

    /// Puts the session into the KV.
    ///
    /// **THIS DOES NOT CREATE THE SESSION USING CLOUDFLARE'S API.**
    /// That happens in the constructor.
    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), KvError> {
        let data = RealtimeSessionData {
            session_id: self.session_id.clone(),
            data_tracks: self.session_data_tracks
                .is_empty().then(|| self.session_data_tracks.clone())
        };

        let kv = env.kv("kSESSIONS").unwrap();

        kv.put(&*format!("rt:{}", self.user_id.clone()), data)
            .unwrap()
            .expiration_ttl(RT_SESSION_MAX_AGE)
            .execute()
            .await
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        kv.delete(&*format!("turn:{}", self.user_id))
            .await
    }

    /// Checks if the session is still alive.
    /// If it is, Ok(Some(self)) is returned.
    /// If it isn't, Ok(None) is returned.
    /// If an error occurs, Err((self, ERROR)) is returned.
    async fn check(&self, env: Arc<Env>) -> Result<bool, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let res = kv.get(&*format!("turn:{}", self.user_id))
            .cache_ttl(KV_CACHE_TTL)
            .json::<RealtimeSessionData>()
            .await;

        res.map(|v| v.is_some())
    }
}
