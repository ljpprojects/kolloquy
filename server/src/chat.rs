use crate::data::{KolloquyDB, R2Query, KOLLOQUY_CHATS_BUCKET, USER_AVATAR_BUCKET};
use crate::random_user_id;
use crate::user::{User, UserQuery};
use brotli::{BrotliCompress, BrotliDecompress};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::ops::Deref;
use svg::Document;

#[derive(Serialize, Deserialize)]
pub struct CreateChatBody {
    pub participants: Vec<String>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SocketChatBody {
    pub content: Option<String>,
    pub action: String,
    pub author: SocketChatAuthor,
    pub chat: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SocketChatAuthor {
    pub avatar: String,
    pub id: String,
    pub is_self: bool,
    pub handle: String,
}

pub enum ChatQuery<'a> {
    /// Upload the chat to the R2 bucket
    PutChat,
    
    /// Add a message to the local chat
    AddMessage(Message),
    
    /// Add a participant to the chat
    AddParticipant(&'a mut User),
    
    /// Remove a participant from the chat
    RemoveParticipant(&'a mut User),

    PutIcon(Document),
    
    /// Delete the chat
    Delete,
}

pub fn create_chat_icon() -> Document {
    let mut random = rand::rng();

    let hue = random.random_range(0..360);
    let sat = random.random_range(75..100);
    let lit = random.random_range(40..50);

    let wrapping_add = |x: i32, y: i32, thresh: i32| {
        if x + y > thresh {
            x + y - thresh
        } else {
            x + y
        }
    };

    let gradient = format!("linear-gradient(135deg, hsl({hue}deg, {sat}%, {lit}%), hsl({}deg, {sat}%, {lit}%))", wrapping_add(hue, 50, 360));

    let fe_turbelence = svg::node::element::FilterEffectTurbulence::new()
        .set("type", "fractalNoise")
        .set("baseFrequency", "5")
        .set("numOctaves", "3")
        .set("stitchTiles", "noStitch");

    let filter = svg::node::element::Filter::new()
        .set("id", "noise")
        .add(fe_turbelence);

    let svg = Document::new()
        .set("width", "100px")
        .set("height", "100px")
        .set("style", format!("background: {gradient}"))
        .add(filter)
        .add(
            svg::node::element::Rectangle::new()
                .set("filter", "url(#noise)")
                .set("opacity", "90%")
                .set("height", "100%")
                .set("width", "100%")
        )
        .add(
            svg::node::element::Rectangle::new()
                .set("x", "25%")
                .set("y", "25%")
                .set("height", "50%")
                .set("width", "50%")
                .set("fill", "#e9eaff")
        );

    svg
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Message {
    /// This contains the current message in the chat in LIFO order (last sent message is first in the vec)
    pub content: Vec<String>,
    pub author: String,
    pub sent: DateTime<Utc>,
    pub id: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Chat {
    pub name: String, 
    pub id: String,
    pub icon_url: String,
    pub messages: Vec<Message>,
    remote_url: String,
}

impl<'a> Chat {
    /// Create a new blank chat and its icon
    pub async fn new(name: String) -> (Self, Document) {
        let id = random_user_id().await;
        let icon_url = format!("/chats/{id}.svg.br");
        let remote_url = format!("/{id}.json.br");
        
        (Self {
            name,
            id,
            icon_url,
            remote_url,
            messages: Vec::new(),
        }, create_chat_icon())
    }
    
    pub async fn execute(&mut self, query: &mut ChatQuery<'a>) {
        match query {
            ChatQuery::PutChat => {
                let mut serialised = Cursor::new(serde_json::to_string(self).unwrap());
                let mut compressed = Vec::new();
                
                BrotliCompress(&mut serialised, &mut compressed, &Default::default()).unwrap();
                
                KOLLOQUY_CHATS_BUCKET.deref()
                    .put_object(self.remote_url.as_str(), &*compressed).await.unwrap();
            }
            
            ChatQuery::AddMessage(message) => {
                self.messages.push(message.clone());
            }
            
            ChatQuery::AddParticipant(user) => {
                user.enrolled_chats.push(self.id.clone());
                
                let db = KolloquyDB::new();
                let query = UserQuery::UpdateRemote(user.clone().clone());
                
                db.execute(&query).await.unwrap();
            }
            
            ChatQuery::RemoveParticipant(user) => {
                user.enrolled_chats.retain(|chat| chat != &self.id);

                let db = KolloquyDB::new();
                let query = UserQuery::UpdateRemote(user.clone().clone());

                db.execute(&query).await.unwrap();
            }

            ChatQuery::PutIcon(icon) => {
                let mut serialised = Cursor::new(icon.to_string());
                let mut compressed = Vec::new();

                BrotliCompress(&mut serialised, &mut compressed, &Default::default()).unwrap();

                USER_AVATAR_BUCKET.deref()
                    .put_object(self.icon_url.as_str(), &*compressed).await.unwrap();
            }
            
            ChatQuery::Delete => {
                KOLLOQUY_CHATS_BUCKET.deref()
                    .delete_object(self.remote_url.as_str()).await.unwrap();
            }
        }
    }
    
    pub async fn from_remote(id: String) -> Option<Self> {
        eprintln!("REM URL /{id}.json.br");

        let remote_url = format!("/{id}.json.br");

        let mut compressed = Cursor::new(KOLLOQUY_CHATS_BUCKET.deref()
            .get_object(remote_url.as_str()).await.ok()?
            .to_vec());

        let mut serialised = Cursor::new(Vec::new());

        BrotliDecompress(&mut compressed, &mut serialised).unwrap();

        println!("{}", String::from_utf8_lossy(&serialised.clone().into_inner()));

        let chat = serde_json::from_str(&String::from_utf8_lossy(&serialised.clone().into_inner())).unwrap();

        chat
    }
}

mod tests {
    use super::*;
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_upload_retrieve() {
        dotenv().ok();

        let (mut chat, _) = Chat::new("Test Chat".to_string()).await;

        chat.execute(&mut ChatQuery::PutChat).await;

        println!("{chat:?}");

        let chat = Chat::from_remote(chat.id.clone()).await;

        println!("{chat:?}");
    }
}