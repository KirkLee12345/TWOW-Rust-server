use lettre::message::{header::ContentType, Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};

pub struct EmailSender {
    from_email: String,
    email_authentication: String,
}

impl EmailSender {
    pub fn new(from_email: String, email_authentication: String) -> Self {
        EmailSender {
            from_email,
            email_authentication,
        }
    }

    pub fn send_email(&self, to: &str, subject: &str, main_data: &str) -> Result<(), Box<dyn std::error::Error>> {
        let email = Message::builder()
            .from(
                Mailbox::new(
                    Some("兵者TWOW".to_string()),
                    self.from_email.parse()?,
                )
            )
            .to(Mailbox::new(None, to.parse()?))
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(main_data.to_string())?;

        let creds = Credentials::new(
            self.from_email.clone(),
            self.email_authentication.clone(),
        );

        let mailer = SmtpTransport::relay("smtp.qq.com")?
            .credentials(creds)
            .build();

        match mailer.send(&email) {
            Ok(_) => println!("邮件发送成功至: {}", to),
            Err(e) => eprintln!("邮件发送失败: {}", e),
        }

        Ok(())
    }
}
