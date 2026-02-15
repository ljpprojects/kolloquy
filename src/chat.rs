use alloc::{borrow::ToOwned, string::String, sync::Arc, vec::Vec};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{Date, DateTime, Utc};
use futures_lite::{StreamExt, stream};
use serde::Deserialize;
use worker::Env;

use crate::{d1::{D1Interface, D1InterfaceExt}, user::{User, UserIdentifyingKey}};

pub struct Chat {
    id: String,
    participants: Vec<User>,
    name: String,
    secret: [u8; 24],
    creation_timestamp: DateTime<Utc>,
}

#[derive(Deserialize)]
struct DBRow {
    id: String,
    participants: String,
    name: String,
    secret: String,
    creation_timestamp: i64,
}

impl Chat {
    pub fn add_participant(&mut self, participant: User) -> &mut User {
        let user = self.participants.push_mut(participant);

        user.participations.push(self.id.clone());
        user
    }

    pub fn remove_participant(&mut self, participant: User) -> Option<User> {
        let mut user = self.participants.swap_remove(self.participants.iter().position(|u| participant.id == u.id)?);

        // The stored participa
        user.participations.swap_remove(user.participations.iter().position(|c| self.id.eq(c))?);
        Some(user)
    }

    pub fn invite(&mut self, from: &User, invitee: &mut User) {
        invitee.pending_entrances.push((from.id.clone(), self.id.clone()));
    }
}

impl D1Interface for Chat {
    type Key = String;

    async fn fetch_from_remote(id: &Self::Key, env: Arc<Env>) -> Result<Option<Self>, worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = kdb.prepare("select * from chats where id = ?1")
            .bind(&[(&*id).into()])?;

        let Some(row) = query.first::<DBRow>(None).await? else {
            return Ok(None)
        };

        let participants = stream::iter(
            row.participants
                .split(",")
                .map(ToOwned::to_owned)
            ).then(|id|  User::owned_fetch_from_remote(UserIdentifyingKey::UserId(id), env.clone()))
            .filter(|r| r.as_ref().map(|m| m.is_some()).unwrap_or(true))
            .map(|r| r.map(Option::unwrap))
            .collect::<Vec<Result<User, worker::Error>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<User>, worker::Error>>()?;

        Ok(Some(Self {
            id: row.id,
            name: row.name,
            secret: BASE64_STANDARD.decode(row.secret).unwrap().try_into().unwrap(),
            participants,
            creation_timestamp: DateTime::from_timestamp_millis(row.creation_timestamp).unwrap(),
        }))
    }

    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = kdb.prepare("insert into chats values (?1, ?2, ?3, ?4, ?5)")
            .bind(&[
                (&*self.id).into(),
                (&*self.participants.iter().map(|u| u.id.clone()).collect::<Vec<_>>().join(",")).into(),
                (&*self.name).into(),
                (&*BASE64_STANDARD.encode(self.secret)).into(),
                (self.creation_timestamp.timestamp_millis()).into(),
            ])?;

        query.run().await.map(|r| r.results::<Vec<u8>>()).flatten().map(|_| ())
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = kdb.prepare("delete from chats where id = ?1")
            .bind(&[(&*self.id).into()])?;

        query.run().await.map(|r| r.results::<Vec<u8>>()).flatten().map(|_| ())
    }
}
