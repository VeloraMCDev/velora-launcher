use crate::{settings::ConnectionsSettings, Error};
use lettre::{message::Mailbox, transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

pub fn message(settings: &ConnectionsSettings, to: &str, subject: &str, body: &str, html: Option<&str>) -> Result<Message, Error> {
    if settings.resend_api_key.is_empty() || settings.sender_email.is_empty() {
        return Err(Error::request("configure Resend SMTP and a sender email in Settings first"));
    }
    let from: Mailbox = if settings.sender_name.is_empty() {
        settings.sender_email.parse()
    } else {
        format!("{} <{}>", settings.sender_name, settings.sender_email).parse()
    }
    .map_err(|_| Error::request("invalid sender email"))?;
    let to: Mailbox = to.parse().map_err(|_| Error::request("invalid recipient email"))?;
    let builder = Message::builder().from(from).to(to).subject(subject);
    match html {
        Some(html) => builder.multipart(lettre::message::MultiPart::alternative_plain_html(body.to_string(), html.to_string())),
        None => builder.body(body.to_string()),
    }
    .map_err(|_| Error::request("invalid email message"))
}
/// One SMTP acceptance attempt. Caller owns queue, recipient scope and retry policy.
pub async fn send(settings: &ConnectionsSettings, to: &str, subject: &str, body: &str, html: Option<&str>) -> Result<(), Error> {
    let message = message(settings, to, subject, body, html)?;
    let smtp = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.resend.com")
        .map_err(|error| Error::request(format!("SMTP configuration failed: {error}")))?
        .credentials(Credentials::new("resend".into(), settings.resend_api_key.clone()))
        .build();
    smtp.send(message).await.map_err(|error| Error::gateway(format!("Resend SMTP rejected the email: {error}")))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plaintext_and_alternative_html_preserve_sender_recipient_and_message_parts() {
        let settings = ConnectionsSettings {
            resend_api_key: "synthetic-mail-key".into(),
            sender_email: "sender@example.invalid".into(),
            sender_name: "Velora".into(),
            ..Default::default()
        };
        let plain = String::from_utf8(
            message(&settings, "player@example.invalid", "Synthetic subject", "Synthetic plain body", None).unwrap().formatted(),
        )
        .unwrap();
        assert!(plain.contains("From: Velora <sender@example.invalid>"));
        assert!(plain.contains("To: player@example.invalid"));
        assert!(plain.contains("Subject: Synthetic subject"));
        assert!(plain.ends_with("Synthetic plain body"));
        assert!(!plain.contains("synthetic-mail-key"));
        let html = String::from_utf8(
            message(&settings, "player@example.invalid", "Synthetic subject", "Synthetic plain body", Some("<p>Synthetic HTML body</p>"))
                .unwrap()
                .formatted(),
        )
        .unwrap();
        for part in ["multipart/alternative", "text/plain", "text/html", "Synthetic plain body", "<p>Synthetic HTML body</p>"] {
            assert!(html.contains(part), "missing {part}");
        }
    }
    #[tokio::test]
    async fn configuration_and_mailbox_errors_keep_precedence_without_external_delivery() {
        let mut settings = ConnectionsSettings::default();
        assert_eq!(
            send(&settings, "invalid", "Synthetic", "body", None).await.unwrap_err().message,
            "configure Resend SMTP and a sender email in Settings first"
        );
        settings.resend_api_key = "synthetic-mail-key".into();
        settings.sender_email = "invalid".into();
        assert_eq!(message(&settings, "invalid", "Synthetic", "body", None).unwrap_err().message, "invalid sender email");
        settings.sender_email = "sender@example.invalid".into();
        assert_eq!(message(&settings, "invalid", "Synthetic", "body", None).unwrap_err().message, "invalid recipient email");
    }
}
