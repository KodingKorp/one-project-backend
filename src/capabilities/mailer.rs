use std::str::FromStr;

use handlebars::Handlebars;
use lettre::{message::{header::ContentType, Mailbox}, transport::smtp::{authentication::Credentials, PoolConfig}, Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use super::config;

#[derive(Clone)]
pub struct Mailer {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    default_sender: Mailbox,
}

impl Mailer {
    pub fn new() -> Self {
        let smtp_server = config::get_env::<String>("SMTP");
        let smtp_port = config::get_env("SMTP_PORT");
        let smtp_user_name = config::get_env("SMTP_UNAME");
        let smtp_user_pass= config::get_env("SMTP_PASS");
        let default_sender = config::get_env::<String>("DEFAULT_SENDER");
        let default_sender_name = config::get_env("DEFAULT_SENDER_NAME");

        let creds = Credentials::new(smtp_user_name, smtp_user_pass);

        let mut split = default_sender.split("@");
        let user = split.next().unwrap();
        let domain = split.next().unwrap();

        let sender = Address::new(user, domain).unwrap();
        let transport:AsyncSmtpTransport::<Tokio1Executor>;
        let pool_config = PoolConfig::new().min_idle(1);

        if config::get_env::<String>("ENV") == "production" {
            transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_server).unwrap()
                .pool_config(pool_config)
                .port(smtp_port)
                .credentials(creds)
                .build();
        } else {
            transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_server)
            .pool_config(pool_config)
            .port(smtp_port)
            .build();
        }
        Self {
            mailer: transport,
            default_sender: Mailbox::new(Some(default_sender_name), sender),
        }
    }

    pub fn render_template(&self, template_name: &str, data: serde_json::Value) -> Result<String, handlebars::RenderError> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_file(template_name, &format!("./templates/{}.hbs", template_name))?;
        handlebars.register_template_file("styles", "./templates/partials/styles.hbs")?;
        handlebars.register_template_file("base", "./templates/layout/base.hbs")?;
        let content_template = handlebars.render(template_name, &data)?;

        Ok(content_template)
    }

    pub async fn send_email(
        &self,
        template_name: &str,
        subject: &str,
        to_name: &str,
        to_email: &str,
        data: serde_json::Value
    ) -> Result<(), Box<dyn std::error::Error>> {

        let to = Address::from_str(to_email)?;
        let html_template = self.render_template(template_name, data)?;
        let email = Message::builder()
            .to(
                Mailbox::new(Some(to_name.to_owned()), to)
            )
            .reply_to(self.default_sender.clone())
            .from(self.default_sender.clone())
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_template.clone())?;
        
        tracing::debug!("Before mail send");
        let _r = self.mailer.send(email).await;
        tracing::debug!("After mail send");
        Ok(())
    }

    pub async fn check_connection(&self) -> bool {
        match self.mailer.test_connection().await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("{:#?}",e);
                false
            }
        }
    }
    
}
