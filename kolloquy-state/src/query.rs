use std::sync::Arc;

use worker::{D1Database, D1Result, Env, wasm_bindgen::JsValue};

pub const DB_BINDING_NAME: &str = "kDB";

pub async fn query_db<T>(query: (T, &[JsValue]), env: Arc<Env>) -> Result<D1Result, worker::Error>
where
    T: Into<String>
{
    let db: D1Database = env.d1(DB_BINDING_NAME)?;
    let query = db
        .prepare(query.0)
        .bind(query.1)?;

    query.all().await
}