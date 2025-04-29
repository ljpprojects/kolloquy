use crate::data::{DBQuery, R2Query, R2QueryKind};
use base64::alphabet::Alphabet;
use base64::engine::GeneralPurpose;
use base64::Engine;
use brotli::BrotliCompress;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use poem::http::Uri;
use reqwest::Url;
use svg::Document;
use crate::chat::Chat;

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub enrolled_chats: Vec<String>
}

#[derive(Serialize, Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub handle: String,
    pub age: u8,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthenticateBody {
    pub email: String,
    pub password: String,
    pub redirect: String,
}

pub enum UserQuery {
    GetByEmail(String),
    GetByHandle(String),
    GetByID(String),
    PutToDB(User),
    UploadAvatar(User, Document),
    GetAvatar(User),
    UpdateRemote(User),
}

impl DBQuery for UserQuery {
    fn to_sql_query_string(&self) -> (String, Vec<String>) {
        match self {
            Self::GetByEmail(email) => ("SELECT * FROM users WHERE email = ?".to_string(), vec![email.clone()]),
            Self::GetByHandle(handle) => ("SELECT * FROM users WHERE handle = ?".to_string(), vec![handle.clone()]),
            Self::GetByID(id) => ("SELECT * FROM users WHERE userid = ?".to_string(), vec![id.clone()]),
            Self::PutToDB(user) => {
                let mut read_desc = Cursor::new(user.description.clone());
                let mut compressed_desc = Vec::with_capacity((user.description.len() as f64 / 1.3).ceil() as usize);

                BrotliCompress(&mut read_desc, &mut compressed_desc, &Default::default()).unwrap();

                (
                    "INSERT INTO users VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)".to_string(),
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
                        user.enrolled_chats.join(",")
                    ]
                )
            },

            Self::UpdateRemote(user) => {
                let mut read_desc = Cursor::new(user.description.clone());
                let mut compressed_desc = Vec::with_capacity((user.description.len() as f64 / 1.3).ceil() as usize);

                BrotliCompress(&mut read_desc, &mut compressed_desc, &Default::default()).unwrap();

                (
                    "UPDATE users\nSET email = ?, handle = ?, password = ?, age = ?, country = ?, preferences = ?, suspended = ?, age_verified = ?, userid = ?, phone_number = ?, joined = ?, description = ?, last_agent = ?, last_approx_country = ?, avatar_url = ?, email_verified = ?, last_login = ?, failed_login_attempts = ?, locked_until = ?, timezone = ?, enrolled_chats = ?\nWHERE userid = ?;".to_string(),
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
                        user.enrolled_chats.join(","),
                        user.user_id.clone()
                    ]
                )
            },
            
            _ => panic!("Cannot convert to SQL query string for this query type.")
        }
    }
    
    fn has_result(&self) -> bool {
        match self {
            Self::GetByEmail(_) | Self::GetByHandle(_) | Self::GetByID(_) => true,
            _ => false
        }
    }
}

impl R2Query for UserQuery {
    fn path(&self) -> String {
        match self {
            Self::UploadAvatar(user, _) | Self::GetAvatar(user) => {
                format!("/{}", user.avatar_url)
            }
            _ => panic!("Cannot make R2 query for this query type.")
        }
    }

    fn kind(&self) -> R2QueryKind {
        match self {
            Self::UploadAvatar(user, svg) => {
                let mut avatar = Cursor::new(svg.to_string());
                let mut compressed_avatar = Vec::with_capacity((svg.to_string().len() as f64 / 1.3).ceil() as usize);
                BrotliCompress(&mut avatar, &mut compressed_avatar, &Default::default()).unwrap();
                 
                R2QueryKind::PutObject(compressed_avatar)
            }
            Self::GetAvatar(_) => {
                R2QueryKind::GetObject
            }
            _ => panic!("Cannot make R2 query for this query type.")
        }
    }
    
    fn has_result(&self) -> bool {
        match self {
            UserQuery::GetByEmail(_) | UserQuery::GetByHandle(_) | UserQuery::GetByID(_) => true,
            UserQuery::GetAvatar(_) => true,
            _ => false
        }
    }
}