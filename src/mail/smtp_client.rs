use crate::config::AccountConfig;
use crate::mail::error::MailError;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use log::info;

pub struct SmtpClient;

impl SmtpClient {
    pub async fn send(
        config: &AccountConfig,
        to: &str,
        cc: &str,
        bcc: &str,
        subject: &str,
        body: &str,
        attachments: &[(String, Vec<u8>)],
    ) -> Result<Vec<u8>, MailError> {
        let from = format!("{} <{}>", config.display_name, config.email)
            .parse()
            .map_err(|e| MailError::Smtp(format!("Invalid from address: {e}")))?;

        let to_addr = to.parse().map_err(|e| MailError::Smtp(format!("Invalid to address: {e}")))?;

        let mut msg_builder = Message::builder()
            .from(from)
            .to(to_addr)
            .subject(subject);

        for addr in cc.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if let Ok(parsed) = addr.parse() {
                msg_builder = msg_builder.cc(parsed);
            }
        }

        for addr in bcc.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if let Ok(parsed) = addr.parse() {
                msg_builder = msg_builder.bcc(parsed);
            }
        }

        // Build body: if body contains HTML tags, send as multipart/alternative
        let has_html = body.contains("<b>")
            || body.contains("<i>")
            || body.contains("<a ")
            || body.contains("<br>");

        let message = if attachments.is_empty() && !has_html {
            // Plain text only
            msg_builder
                .header(ContentType::TEXT_PLAIN)
                .body(body.to_string())
                .map_err(|e| MailError::Smtp(format!("Failed to build message: {e}")))?
        } else {
            // Build multipart
            let text_body = strip_html_tags(body);
            let html_body = if has_html {
                format!(
                    "<html><body style=\"font-family:sans-serif\">{}</body></html>",
                    body.replace('\n', "<br>")
                )
            } else {
                body.replace('\n', "<br>")
            };

            let alternative = MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(text_body),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body),
                );

            if attachments.is_empty() {
                msg_builder
                    .multipart(alternative)
                    .map_err(|e| MailError::Smtp(format!("Failed to build message: {e}")))?
            } else {
                let mut mixed = MultiPart::mixed().multipart(alternative);
                for (filename, data) in attachments {
                    let content_type =
                        ContentType::parse("application/octet-stream").unwrap_or(ContentType::TEXT_PLAIN);
                    let attachment = Attachment::new(filename.clone())
                        .body(data.clone(), content_type);
                    mixed = mixed.singlepart(attachment);
                }
                msg_builder
                    .multipart(mixed)
                    .map_err(|e| MailError::Smtp(format!("Failed to build message: {e}")))?
            }
        };

        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
            .map_err(|e| MailError::Smtp(format!("SMTP relay error: {e}")))?
            .port(config.smtp_port)
            .credentials(creds)
            .build();

        let raw_message = message.formatted();

        mailer
            .send(message)
            .await
            .map_err(|e| MailError::Smtp(format!("Send failed: {e}")))?;

        info!("Email sent successfully to {to}");
        Ok(raw_message)
    }
}

fn strip_html_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_plain_text() {
        assert_eq!(strip_html_tags("hello world"), "hello world");
    }

    #[test]
    fn strip_simple_tags() {
        assert_eq!(strip_html_tags("<b>bold</b>"), "bold");
    }

    #[test]
    fn strip_nested_tags() {
        assert_eq!(
            strip_html_tags("<div><b>text</b></div>"),
            "text"
        );
    }

    #[test]
    fn strip_tags_with_attributes() {
        assert_eq!(
            strip_html_tags(r#"<a href="https://example.com">link</a>"#),
            "link"
        );
    }

    #[test]
    fn strip_mixed_content() {
        assert_eq!(
            strip_html_tags("Hello <b>bold</b> and <i>italic</i> text"),
            "Hello bold and italic text"
        );
    }

    #[test]
    fn strip_empty() {
        assert_eq!(strip_html_tags(""), "");
    }

    #[test]
    fn strip_only_tags() {
        assert_eq!(strip_html_tags("<br><hr>"), "");
    }

    #[test]
    fn strip_unclosed_tag() {
        assert_eq!(strip_html_tags("<b>hello"), "hello");
    }
}
