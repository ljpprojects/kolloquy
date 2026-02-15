use alloc::{format, string::{String, ToString}, sync::Arc, vec::Vec};
use const_format::formatcp;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::JsValue;
use worker::{CfProperties, Env, KvError, Method, Request, RequestInit};

pub const TURN_API_BASE: &str = "https://rtc.live.cloudflare.com/v1/turn";

#[derive(Deserialize, Serialize, Clone)]
pub struct IceServer {
    urls: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    credential: Option<String>
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TURNApiConfig {
    #[serde(rename = "iceServers")]
    ice_servers: Vec<IceServer>
}

#[derive(Clone)]
pub struct TURNSession {
    user_id: String,
    config: TURNApiConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TURNSessionData {
    config: TURNApiConfig,
}

pub const KV_CACHE_TTL: u64 = 12 * 60 * 60;
pub const TURN_SESSION_MAX_AGE: u64 = 24 * 60 * 60;

impl TURNSession {
    pub async fn new(user_id: String, env: Arc<Env>) -> worker::Result<Self> {
        let cf_turn_tk_id = env.secret("CF_TURN_TK_ID").unwrap().to_string();
        let cf_turn_api_key = env.secret("CF_TURN_API_KEY").unwrap().to_string();

        let headers = worker::Headers::new();

        headers.set("Authorization", &*cf_turn_api_key).unwrap();
        headers.set("Content-Type", "application/json").unwrap();

        let body = json!({
            "ttl": 86400,
        });

        let mut req_init = RequestInit::new();
        req_init
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(JsValue::from_str(&*body.to_string())));

        let request = Request::new_with_init(
            &*format!("{TURN_API_BASE}/keys/{cf_turn_tk_id}/credentials/generate-ice-servers"),
            &req_init
        ).unwrap();

        let mut res = worker::Fetch::Request(request).send().await?;

        let config = res.json::<TURNApiConfig>().await?;

        Ok(Self {
            user_id,
            config,
        })
    }

    pub async fn from_user_id(user_id: String, env: Arc<Env>) -> Result<Option<Self>, worker::Error> {
        let kv = env.kv("kSESSIONS").unwrap();

        let Some(data) = kv.get(&*format!("turn:{user_id}"))
            .cache_ttl(KV_CACHE_TTL)
            .json::<TURNSessionData>()
            .await?
        else {
            return Ok(None)
        };

        Ok(Some(TURNSession {
            user_id,
            config: data.config,
        }))
    }

    pub async fn put_to_remote(self, env: Arc<Env>) -> Result<Self, KvError> {
        let data = TURNSessionData {
            config: self.config.clone(),
        };

        let kv = env.kv("kSESSIONS").unwrap();

        kv.put(&*format!("turn:{}", self.user_id.clone()), data)
            .unwrap()
            .expiration_ttl(TURN_SESSION_MAX_AGE)
            .execute()
            .await
            .map(|_| self)
    }

    /// Deletes the session from the remote Kv and revokes the credentials
    pub async fn delete_from_remote(self, env: Arc<Env>) -> worker::Result<()> {
        let kv = env.kv("kSESSIONS").unwrap();

        kv.delete(&*format!("turn:{}", self.user_id))
            .await?;

        let cf_turn_tk_id = env.secret("CF_TURN_TK_ID").unwrap().to_string();
        let cf_turn_api_key = env.secret("CF_TURN_API_KEY").unwrap().to_string();

        let headers = worker::Headers::new();

        headers.set("Authorization", &*cf_turn_api_key).unwrap();
        headers.set("Content-Type", "application/json").unwrap();

        let mut req_init = RequestInit::new();
        req_init
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(None);

        let request = Request::new_with_init(
            &*format!("{TURN_API_BASE}/keys/{cf_turn_tk_id}/credentials/{}/revoke", self.config.ice_servers[1].username.as_ref().unwrap()),
            &req_init
        ).unwrap();

        let mut res = worker::Fetch::Request(request).send().await?;

        let _body = res.bytes().await?;

        Ok(())
    }

    /// Checks if the session is still alive.
    /// If it is, Ok(Some(self)) is returned.
    /// If it isn't, Ok(None) is returned.
    /// If an error occurs, Err((self, ERROR)) is returned.
    pub async fn check(self, env: Arc<Env>) -> Result<Option<Self>, (Self, KvError)> {
        let kv = env.kv("kSESSIONS").unwrap();

        let res = kv.get(&*format!("turn:{}", self.user_id))
            .cache_ttl(KV_CACHE_TTL)
            .json::<TURNSessionData>()
            .await;

        match res {
            Err(e) => Err((self, e)),
            Ok(Some(_)) => Ok(Some(self)),
            Ok(None) => Ok(None)
        }
    }
}
