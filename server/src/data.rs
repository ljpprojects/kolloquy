use crate::user::User;
use chrono::DateTime;
use s3::error::S3Error;
use s3::request::ResponseData;
use s3::{Bucket, Region};
use serde_json::{json, Map, Value};
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::LazyLock;
use awscreds::Credentials;

pub static USER_AVATAR_BUCKET: LazyLock<Box<Bucket>, fn() -> Box<Bucket>> = LazyLock::new(|| Bucket::new(
    "kolloquy-user-avatars",
    Region::R2 { account_id: env::var("CLOUDFLARE_ACC_ID").unwrap() },
    Credentials::new(
        Some(&*env::var("R2_ACCESS_KEY").unwrap()),
        Some(&*env::var("R2_SECRET_KEY").unwrap()),
        None,
        None,
        None,
    ).unwrap(),
).unwrap().with_path_style());

pub static KOLLOQUY_CHATS_BUCKET: LazyLock<Box<Bucket>, fn() -> Box<Bucket>> = LazyLock::new(|| Bucket::new(
    "kolloquy-chats",
    Region::R2 { account_id: env::var("CLOUDFLARE_ACC_ID").unwrap() },
    Credentials::new(
        Some(&*env::var("R2_ACCESS_KEY").unwrap()),
        Some(&*env::var("R2_SECRET_KEY").unwrap()),
        None,
        None,
        None,
    ).unwrap(),
).unwrap().with_path_style());

pub enum R2QueryKind {
    PutObject(Vec<u8>),
    GetObject,
    DeleteObject,
}

pub trait Query: Send + Sync {
    fn has_result(&self) -> bool;
}

pub trait R2Query: Query {
    fn path(&self) -> String;
    fn kind(&self) -> R2QueryKind;
}

pub trait DBQuery: Query {
    fn to_sql_query_string(&self) -> (String, Vec<String>);
}

pub struct KolloquyDB<'a>(PhantomData<&'a ()>);

pub struct KolloquyR2 {
    bucket: Bucket
}

#[derive(Debug)]
pub enum QueryError<'a> {
    InvalidQuery,
    NotFound,
    ServerError,
    Other(Box<dyn Error + 'a>),
}

// Just trust me bro
unsafe impl<'a> Send for QueryError<'a> {}
unsafe impl<'a> Sync for QueryError<'a> {}

impl<'a> Clone for QueryError<'a> {
    fn clone(&self) -> Self {
        match self {
            Self::InvalidQuery => Self::InvalidQuery,
            Self::NotFound => Self::NotFound,
            Self::ServerError => Self::ServerError,
            Self::Other(e) => unsafe {
                Self::Other((e as *const Box<dyn Error + 'a>).read())
            },
        }
    }
}

impl<'a, E: Error> From<&'a E> for QueryError<'a> {
    fn from(err: &'a E) -> Self {
        Self::Other(Box::new(err))
    }
}

impl<'a> From<&'a dyn Error> for QueryError<'a> {
    fn from(err: &'a dyn Error) -> Self {
        Self::Other(Box::new(err))
    }
}

impl<'a, E: Error + 'a> From<Box<E>> for QueryError<'a> {
    fn from(err: Box<E>) -> Self {
        Self::Other(err.into())
    }
}

impl<'a> From<Box<dyn Error + 'a>> for QueryError<'a> {
    fn from(err: Box<dyn Error + 'a>) -> Self {
        Self::Other(err)
    }
}

impl Display for QueryError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for QueryError<'_> {}

impl<'a> KolloquyDB<'a> {
    pub fn new() -> Self {
        Self(PhantomData)
    }

    pub async fn execute<Q: DBQuery>(&self, original_query: &Q) -> Result<Option<User>, QueryError<'a>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/d1/database/{}/query",
            env::var("CLOUDFLARE_ACC_ID").unwrap(),
            env::var("KOLLOQUY_DB_ID").unwrap()
        );
        
        let client = reqwest::Client::new();

        let (query, params) = original_query.to_sql_query_string();

        let json: Map<String, Value> = serde_json::from_str(&*client.post(url)
            .header("Content-Type", "application/json")
            .header("X-Auth-Email", env::var("CLOUDFLARE_EMAIL").unwrap())
            .header("Authorization", format!("Bearer {}", env::var("CLOUDFLARE_API_KEY").unwrap()))
            .body(serde_json::to_string(&json!({
                "sql": query,
                "params": params,
            })).unwrap())
            .send()
            .await
            .map_err(|e| QueryError::Other(Box::new(e)))?
            .text()
            .await
            .map_err(|e| QueryError::Other(Box::new(e)))?).unwrap();

        if !json.get("success").unwrap().as_bool().unwrap() {
            eprintln!("{json:#?}");

            return Err(QueryError::ServerError);
        }
        
        if !original_query.has_result() {
            return Ok(None)
        }
        
        if json.get("result").unwrap().as_array().unwrap().len() == 0 {
            return Err(QueryError::NotFound);
        }

        let result = json.get("result").unwrap().as_array().unwrap()[0].as_object().unwrap().get("results").unwrap().as_array().unwrap();

        if result.len() == 0 {
            return Err(QueryError::NotFound);
        }

        let results = result[0].as_object().unwrap();

        println!("{results:?}");
        
        let user = User {
            email: results["email"].as_str().unwrap().to_string(),
            handle: results["handle"].as_str().unwrap().to_string(),
            password: results["password"].as_str().unwrap().to_string(),
            age: results["age"].as_number().unwrap().as_u64().unwrap() as i32,
            country: results["country"].as_str().unwrap().to_string(),
            preferences: results["preferences"].as_str().unwrap().to_string(),
            suspended: results["suspended"].as_number().unwrap().as_u64().unwrap() as i32 == 1,
            age_verified: results["age_verified"].as_number().unwrap().as_u64().unwrap() as i32 == 1,
            user_id: results["userid"].as_str().unwrap().to_string(),
            phone_number: results["phone_number"].as_str().unwrap().to_string(),
            joined: DateTime::from_str(results["joined"].as_str().unwrap()).unwrap(),
            description: results["description"].as_str().unwrap().to_string(),
            last_agent: results["last_agent"].as_str().unwrap().to_string(),
            last_approx_country: results["last_approx_country"].as_str().unwrap().to_string(),
            avatar_url: results["avatar_url"].as_str().unwrap().to_string(),
            email_verified: results["email_verified"].as_number().unwrap().as_u64().unwrap() as i32 == 1,
            last_login: DateTime::from_str(results["last_login"].as_str().unwrap()).unwrap(),
            failed_login_attempts: results["failed_login_attempts"].as_number().unwrap().as_u64().unwrap() as i32,
            locked_until: DateTime::from_str(results["locked_until"].as_str().unwrap()).unwrap(),
            timezone: results["timezone"].as_str().unwrap().to_string(),
            enrolled_chats: results["enrolled_chats"].as_str().unwrap().split(",").map(|s| s.to_string()).collect(),
        };

        Ok(Some(user))
    }
}

impl KolloquyR2 {
    pub fn new(bucket: Bucket) -> Self {
        Self {
            bucket
        }
    }

    pub async fn execute<Q: R2Query>(&self, query: &Q) -> Result<ResponseData, S3Error> {
        match query.kind() {
            R2QueryKind::PutObject(data) => {
                self.bucket.put_object(query.path(), &*data).await
            }

            R2QueryKind::GetObject => {
                self.bucket.get_object(query.path()).await
            }

            R2QueryKind::DeleteObject => {
                self.bucket.delete_object(query.path()).await
            }
        }
    }
}

mod tests {
    use crate::create_avatar;
    use crate::data::KolloquyR2;
    use crate::user::{User, UserQuery};
    use awscreds::Credentials;
    use s3::{Bucket, Region};

    #[tokio::test]
    async fn try_upload_file() -> Result<(), Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();
        
        let bucket = Bucket::new(
            "kolloquy-user-avatars",
            Region::R2 { account_id: std::env::var("CLOUDFLARE_ACC_ID").unwrap() },
            // Credentials are collected from environment, config, profile or instance metadata
            Credentials::new(
                Some(&*std::env::var("R2_ACCESS_KEY").unwrap()),
                Some(&*std::env::var("R2_SECRET_KEY").unwrap()),
                None,
                None,
                None,
            )?,
        )?.with_path_style();

        let user = User {
            email: "".to_string(),
            handle: "".to_string(),
            password: "".to_string(),
            age: 0,
            country: "".to_string(),
            preferences: "".to_string(),
            suspended: false,
            age_verified: false,
            user_id: "TEST".to_string(),
            phone_number: "".to_string(),
            joined: Default::default(),
            description: "".to_string(),
            last_agent: "".to_string(),
            last_approx_country: "".to_string(),
            avatar_url: "".to_string(),
            email_verified: false,
            last_login: Default::default(),
            failed_login_attempts: 0,
            locked_until: Default::default(),
            timezone: "".to_string(),
            enrolled_chats: vec![]
        };

        let query = UserQuery::UploadAvatar(user, create_avatar());

        let r2 = KolloquyR2::new(*bucket);

        r2.execute(&query).await?;

        Ok(())
    }
}