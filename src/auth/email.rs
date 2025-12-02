use color_eyre::eyre::{Context, Result};
use lettre::{
	AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
	message::{Mailbox, header::ContentType},
	transport::smtp::authentication::Credentials,
};

use crate::conf::SmtpConfig;

pub struct EmailSender {
	mailer: AsyncSmtpTransport<Tokio1Executor>,
	from: Mailbox,
}

impl EmailSender {
	pub fn new(config: &SmtpConfig) -> Result<Self> {
		let creds = Credentials::new(config.username.clone(), config.password.clone());

		let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
			.context("Failed to create SMTP transport")?
			.port(config.port)
			.credentials(creds)
			.build();

		let from: Mailbox = format!("{} <{}>", config.from_name, config.from_email).parse().context("Invalid from email address")?;

		Ok(Self { mailer, from })
	}

	pub async fn send_verification_email(&self, to_email: &str, username: &str, verification_link: &str) -> Result<()> {
		let to: Mailbox = to_email.parse().context("Invalid recipient email")?;

		let body = format!(
			r#"Hi {username},

Welcome to My Site! Please verify your email address by clicking the link below:

{verification_link}

This link will expire in 24 hours.

If you didn't create an account, you can safely ignore this email.

Best regards,
My Site Team"#
		);

		let email = Message::builder()
			.from(self.from.clone())
			.to(to)
			.subject("Verify your email address")
			.header(ContentType::TEXT_PLAIN)
			.body(body)
			.context("Failed to build email")?;

		self.mailer.send(email).await.context("Failed to send email")?;

		Ok(())
	}
}
