use std::io::Cursor;
use base64::alphabet::Alphabet;
use base64::Engine;
use base64::engine::GeneralPurpose;
use brotli::BrotliCompress;
use brotli::enc::singlethreading::compress_multi;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::data::DBQuery;

pub struct User {
    pub email: String, // user input
    pub handle: String, // user input
    pub password: String, // user input
    pub age: i32, // user input
    pub country: String, // inferred input
    pub preferences: String, // partially user input
    pub suspended: bool, // service input
    pub age_verified: bool, // service input
    pub user_id: String, // random input
    /// unused
    pub phone_number: String, // (unused) user input
    pub joined: DateTime<Utc>, // inferrable input
    pub description: String, // user input
    pub last_agent: String, // inferrable input
    pub last_approx_country: String, // inferrable input
    pub avatar_url: String, // random input, e.g. 
    pub email_verified: bool, // service input
    pub last_login: DateTime<Utc>, // inferrable input
    pub failed_login_attempts: i32, // service input
    pub locked_until: DateTime<Utc>, // service input
    pub timezone: String, // inferrable input
}

#[derive(Serialize, Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub handle: String,
    pub age: u8,
    pub password: String,
}

pub enum UserQuery {
    GetByEmail(String),
    GetByHandle(String),
    GetByID(String),
    Register(User)
}

impl DBQuery for UserQuery {
    fn to_sql_query_string(&self) -> (String, Vec<String>) {
        match self {
            Self::GetByEmail(email) => ("SELECT * FROM users WHERE email = ?".to_string(), vec![email.clone()]),
            Self::GetByHandle(handle) => ("SELECT * FROM users WHERE handle = ?".to_string(), vec![handle.clone()]),
            Self::GetByID(id) => ("SELECT * FROM users WHERE userid = ?".to_string(), vec![id.clone()]),
            Self::Register(user) => {
                let mut read_desc = Cursor::new(user.description.clone());
                let mut compressed_desc = Vec::with_capacity((user.description.len() as f64 / 1.3).ceil() as usize);

                BrotliCompress(&mut read_desc, &mut compressed_desc, &Default::default()).unwrap();

                (
                    "INSERT INTO users VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)".to_string(),
                    vec![
                        user.email.clone(),
                        user.handle.clone(),
                        user.password.clone(),
                        user.age.to_string(),
                        user.country.clone(),
                        user.preferences.clone(),
                        (user.suspended as u8).to_string(),
                        (user.age_verified as u8).to_string(),
                        user.user_id.clone(),
                        user.phone_number.clone(),
                        user.joined.to_rfc3339(),
                        GeneralPurpose::new(&Alphabet::new("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890+/").unwrap(), Default::default()).encode(compressed_desc),
                        user.last_agent.clone(),
                        user.last_approx_country.clone(),
                        user.avatar_url.clone(),
                        (user.email_verified as u8).to_string(),
                        user.last_login.to_rfc3339(),
                        user.failed_login_attempts.to_string(),
                        user.locked_until.to_rfc3339(),
                        user.timezone.to_string(),
                    ]
                )
            }
        }
    }
}