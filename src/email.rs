use alloc::{boxed::Box, string::String, sync::Arc};
use handlebars::Handlebars;
use stackvec::ArrayIntoIter;
use worker::Env;

pub const VERIFICATION_EMAIL_TEMPLATE: &str = include_str!("../assets/verification-email.handlebars");

use crate::{crypto::RandBytesIterator, aws::send_ses_mail};
use serde_json::json;

pub struct VerificationEmail {
    code: [char; 6],
    recipient: String,
    recipient_display_name: String,
}

impl VerificationEmail {
    pub fn new<T: Into<String>, K: Into<String>>(recipient: T, recipient_display_name: K) -> Self {
        let mut digits: [char; 6] = ['\0'; 6];

        for (i, digit) in RandBytesIterator.take(6).enumerate() {
            // We can tolerate a bit of modulo bias
            digits[i] = ((digit % 10) + '0' as u8) as char;
        }

        Self {
            code: digits,
            recipient: recipient.into(),
            recipient_display_name: recipient_display_name.into(),
        }
    }

    pub fn code(&self) -> &[char; 6] {
        &self.code
    }

    pub async fn send(self, env: Arc<Env>) -> Result<(), Box<dyn core::error::Error + Send + Sync>>
    where
        Self: Send + Sync,
    {
        let mut hbars = Handlebars::new();
        hbars.register_template_string("email_verification", VERIFICATION_EMAIL_TEMPLATE).unwrap();

        let data = json!({
            "display_name": self.recipient_display_name,
            "code": ArrayIntoIter::into_iter(self.code).collect::<String>(),
        });

        let content = hbars.render("email_verification", &data).unwrap();

        let subject = "Kolloquy Email Verification";
        let from = "Kolloquy <no-reply@kolloquy.com>";

        send_ses_mail(
            env,
            from,
            &self.recipient,
            subject,
            &content
        ).await?;

        Ok(())
    }
}
