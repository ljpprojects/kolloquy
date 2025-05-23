use crate::user::User;
use futures::SinkExt;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::AsyncTransport;
use lettre::{AsyncSmtpTransport, Message, Tokio1Executor};
use std::env;

pub trait Email {
    fn content(&self) -> String;
    fn subject(&self) -> String;
    fn to(&self) -> Vec<User>;
}

pub enum KolloquyEmail {
    EmailVerify(User),
    Test(User),
    // PasswordReset,
    // MultiFactorAuth,
}

impl Email for KolloquyEmail {
    fn content(&self) -> String {
        match self {
            Self::EmailVerify(user) => todo!(),
            Self::Test(_) => String::from("<b>test</b>"),
        }
    }

    fn subject(&self) -> String {
        match self {
            Self::EmailVerify(user) => todo!(),
            Self::Test(_) => String::from("Test"),
        }
    }

    fn to(&self) -> Vec<User> {
        match self {
            Self::EmailVerify(user) => vec![user.clone()],
            Self::Test(user) => vec![user.clone()],
        }
    }
}

pub struct KolloquyOutboundEmails {
    from: (String, String),
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl KolloquyOutboundEmails {
    pub async fn new(from: (String, String)) -> KolloquyOutboundEmails {
        let creds = Credentials::new("lucasplumb@icloud.com".to_string(), env::var("EMAIL_AUTH_PASSWORD").unwrap());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.mail.me.com")
            .unwrap()
            .credentials(creds)
            .authentication(vec![Mechanism::Plain])
            .build();

        KolloquyOutboundEmails { from, mailer }
    }

    pub async fn send<E: Email>(&mut self, email: &E) {
        let message = Message::builder()
            .from(Mailbox::new(Some(self.from.0.clone()), self.from.1.clone().parse().unwrap()))
            .to(Mailbox::new(Some(email.to()[0].handle.clone()), email.to()[0].email.parse().unwrap()))
            .subject(email.subject())
            .body(email.content())
            .unwrap();

        self.mailer.send(message).await.unwrap();
    }
}

mod tests {
    use crate::data::KolloquyDB;
    use crate::email::{KolloquyEmail, KolloquyOutboundEmails};
    use crate::user::UserQuery;
    use futures::SinkExt;

    #[tokio::test]
    async fn send_no_reply() {
        dotenv::dotenv().ok();

        let mut emailer = KolloquyOutboundEmails::new(("Kolloquy".into(), "lucasplumb@icloud.com".into())).await;

        let db = KolloquyDB::new();
        let query = UserQuery::GetByHandle("ljpprojects".to_string());

        let user = db.execute(&query).await.unwrap().unwrap();

        let email = KolloquyEmail::Test(user);

        emailer.send(&email).await;
    }
}