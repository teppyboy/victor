use mail_parser::{Encoding, HeaderName, MessageParser, decoders::base64};
use minismtp::connection::Mail as SMTPMail;
use rand::random;
use redis::{AsyncCommands, Client, Connection, aio::MultiplexedConnection};
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, error, trace};

use crate::{config::Features, structs::Attachment};

// Database for the mail server
pub struct Database {
    pub client: redis::Client,
    pub connection: redis::Connection,
}

impl Database {
    pub fn new(url: &str) -> Database {
        let client = redis::Client::open(url).expect("Failed to open Redis client.");
        let connection = client
            .get_connection()
            .expect("Failed to connect to Redis.");
        Database { client, connection }
    }
}

#[derive(Debug, Clone)]
pub struct Mail {
    pub features: Features,
    pub db: MultiplexedConnection,
}

impl Mail {
    pub fn new(features: Features, db: MultiplexedConnection) -> Mail {
        Mail { features, db }
    }
    pub async fn handle(mut self, mail: SMTPMail) {
        // This currently doesn't handle spam mails, so fuck
        let recipients = mail.to.clone();
        let sender = mail.from.clone();
        trace!("Recipients: {:#?}", recipients);
        trace!("Sender: {:#?}", sender);
        trace!("Received mail: {:#?}", mail);
        let message = match MessageParser::default().parse(&mail.data) {
            Some(message) => message,
            None => {
                error!("Received empty message, ignoring...");
                return;
            }
        };
        // trace!("Parsed message: {:#?}", message);
        // Parse the mail
        let mut subject: Option<String> = None;
        let mut body_text: Option<String> = None;
        let mut body_html: Option<String> = None;
        let mut attachments: Vec<Attachment> = Vec::new();
        for part in &message.parts {
            for header in &part.headers {
                if header.name == HeaderName::Subject {
                    let value = header.value.as_text().unwrap().to_owned();
                    trace!("Subject: {}", value);
                    subject = Some(value);
                }
            }
            if part.is_text() {
                let text = part.text_contents().unwrap().to_owned();
                if part.is_text_html() {
                    trace!("HTML body: {}", text);
                    body_html = Some(text);
                } else {
                    trace!("Text body: {}", text);
                    body_text = Some(text);
                }
            }
        }
        if message.attachment_count() > 0 && self.features.attachments {
            attachments.reserve_exact(message.attachment_count());
            for msg_attachment in message.attachments() {
                trace!("Attachment: {:?}", msg_attachment);
                let mut file_name = random::<u64>().to_string();
                let mut name: Option<String> = None;
                for header in msg_attachment.headers() {
                    if header.name == HeaderName::ContentType {
                        // Get attachment name
                        let value = header.value().as_content_type().unwrap();
                        let option_name = value.attribute("name");
                        if option_name.is_some() {
                            let l_name = option_name.unwrap().to_owned();
                            name = Some(l_name.clone());
                            trace!("Attachment name: {}", l_name);
                            let option_ext = l_name.split(".").last();
                            if option_ext.is_some() {
                                let ext = option_ext.unwrap();
                                file_name.push('.');
                                file_name.push_str(ext);
                            }
                        }
                    }
                }
                let content: Vec<u8> = if msg_attachment.encoding == Encoding::Base64 {
                    match base64::base64_decode(msg_attachment.contents()) {
                        Some(decoded) => {
                            if decoded.len() == 0 {
                                error!(
                                    "Failed to decode base64 attachment (length 0), ignoring..."
                                );
                                continue;
                            }
                            trace!("Decoded attachment: {:?}", decoded);
                            decoded
                        }
                        None => {
                            error!(
                                "Failed to decode base64 attachment (base64 error), ignoring..."
                            );
                            continue;
                        }
                    }
                } else {
                    msg_attachment.contents().to_owned()
                };
                let attachment = Attachment {
                    name,
                    file_name,
                    content,
                };
                attachments.push(attachment);
            }
        }
        let mut id = random::<u64>().to_string();
        let mut mail_dir = format!("./mails/{}", id);
        loop {
            match fs::try_exists(&mail_dir).await {
                Ok(exists) => {
                    if exists {
                        id = random::<u64>().to_string();
                        mail_dir = format!("./mails/{}", id);
                    } else {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to check if mail directory exists: {}", e);
                    return;
                }
            }
        }
        trace!("Generated ID: {}", id);
        // Save the mail to filesystem
        debug!("Saving mail '{}'...", id);
        trace!("Saving mail to database...");
        // Save bodies info
        let is_body_text = body_text.is_some() && self.features.text_body;
        let is_body_html = body_html.is_some() && self.features.text_body;
        let attachments_len = attachments.len();
        let save_db_cmd_1 = redis::Cmd::hset_multiple(format!("mail:{}", id), &[
            ("has_body_text", is_body_text),
            ("has_body_html", is_body_html),
        ]);
        let save_db_cmd_2 =
            redis::Cmd::hset(format!("mail:{}", id), "attachments", attachments_len);
        let mut conn = self.db.clone();
        match tokio::try_join!(
            save_db_cmd_1.exec_async(&mut conn),
            save_db_cmd_2.exec_async(&mut self.db)
        ) {
            Ok(_) => {
                trace!("Saved mail to database: {}", id);
            }
            Err(e) => {
                error!("Failed to save mail to database: {}", e);
                return;
            }
        }
        // Actually write the mail to filesystem
        match fs::create_dir(&mail_dir).await {
            Ok(_) => {
                trace!("Created mail directory: {}", mail_dir);
            }
            Err(e) => {
                error!("Failed to create mail directory: {}", e);
                return;
            }
        }
        if is_body_text {
            let text_path = format!("{}/body.txt", mail_dir);
            match fs::File::create(&text_path).await {
                Ok(mut file) => {
                    match file.write_all(body_text.unwrap().as_bytes()).await {
                        Ok(_) => {
                            trace!("Wrote text body to file: {}", text_path);
                        }
                        Err(e) => {
                            error!("Failed to write text body to file: {}", e);
                            return;
                        }
                    };
                }
                Err(e) => {
                    error!("Failed to write text body to file: {}", e);
                    return;
                }
            };
        }
        if is_body_html {
            let html_path = format!("{}/body.html", mail_dir);
            match fs::File::create(&html_path).await {
                Ok(mut file) => {
                    match file.write_all(body_html.unwrap().as_bytes()).await {
                        Ok(_) => {
                            trace!("Wrote html body to file: {}", html_path);
                        }
                        Err(e) => {
                            error!("Failed to write html body to file: {}", e);
                            return;
                        }
                    };
                }
                Err(e) => {
                    error!("Failed to write html body to file: {}", e);
                    return;
                }
            };
        }
        if attachments_len > 0 {
            let attachments_path = format!("{}/attachments", mail_dir);
            match fs::create_dir(&attachments_path).await {
                Ok(_) => {
                    trace!("Created attachments directory: {}", attachments_path);
                }
                Err(e) => {
                    error!("Failed to create attachments directory: {}", e);
                    return;
                }
            }
            for attachment in attachments {
                let attachment_path = format!("{}/{}", attachments_path, attachment.file_name);
                match fs::File::create(&attachment_path).await {
                    Ok(mut file) => {
                        match file.write_all(&attachment.content).await {
                            Ok(_) => {
                                trace!("Wrote attachment to file: {}", attachment_path);
                            }
                            Err(e) => {
                                error!("Failed to write attachment to file: {}", e);
                                return;
                            }
                        };
                    }
                    Err(e) => {
                        error!("Failed to write attachment to file: {}", e);
                        return;
                    }
                };
            }
        }
    }
}
