use std::env;
use std::error::Error;
use serde_json::json;
use crate::user::UserQuery;

pub trait R2Query {
    
}

pub trait DBQuery {
    fn to_sql_query_string(&self) -> (String, Vec<String>);
}

pub struct KolloquyDB;

impl KolloquyDB {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute<Q: DBQuery>(&self, query: &Q) -> Result<String, Box<dyn Error>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/d1/database/{}/query",
            env::var("CLOUDFLARE_ACC_ID").unwrap(),
            env::var("KOLLOQUY_DB_ID").unwrap()
        );
        
        let client = reqwest::Client::new();

        let (query, params) = query.to_sql_query_string();

        Ok(client.post(url)
            .header("Content-Type", "application/json")
            .header("X-Auth-Email", env::var("CLOUDFLARE_EMAIL").unwrap())
            .header("Authorization", format!("Bearer {}", env::var("CLOUDFLARE_API_KEY").unwrap()))
            .body(serde_json::to_string(&json!({
                "sql": query,
                "params": params,
            })).unwrap())
            .send()
            .await?
            .text()
            .await?)
    }
}