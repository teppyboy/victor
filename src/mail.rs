use mail_parser::{Encoding, HeaderName, MessageParser, decoders::base64};
use minismtp::connection::Mail as SMTPMail;
use rand::random;
use redis::aio::MultiplexedConnection;
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, error, trace};

use crate::{config::Features, structs::Attachment};

// Database for the mail server
pub struct Database {
    pub client: redis::Client
}

impl Database {
    pub fn new(url: &str) -> Database {
        let client = redis::Client::open(url).expect("Failed to open Redis client.");
        // This shit isn't enough tho, we have to manually test later.
        client
            .get_connection()
            .expect("Failed to connect to Redis.");
        Database { client }
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
    pub async fn handle(self, mail: SMTPMail) {
        // This currently doesn't handle spam mails, so fuck
        let recipients = mail.to.clone();
        // Parse users
        let mut users: Vec<String> = Vec::new();
        users.reserve_exact(recipients.len());
        for recipient in &recipients {
            let username = recipient.split("@").next().unwrap().to_string();
            if username.len() > 24 || username.as_ascii().is_none() {
                trace!("Ignoring invalid username: {}", username);
                continue;
            }
            users.push(username);
        }
        if users.len() == 0 {
            return;
        }
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
        let timestamp = match message.date() {
            Some(date) => date,
            None => {
                error!("Failed to get timestamp from mail, ignoring the mail...");
                return;
            }
        };
        let mut subject: String = "".to_string();
        let mut body_text: Option<String> = None;
        let mut body_html: Option<String> = None;
        let mut attachments: Vec<Attachment> = Vec::new();
        for part in &message.parts {
            for header in &part.headers {
                if header.name == HeaderName::Subject {
                    let value = header.value.as_text().unwrap().to_owned();
                    trace!("Subject: {}", value);
                    subject = value;
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
        let mut body_text_preview = "".to_string();
        let is_body_text = body_text.is_some() && self.features.text_body;
        let is_body_html = body_html.is_some() && self.features.text_body;
        let attachments_len = attachments.len();
        if is_body_text {
            let my_body_text = body_text.clone().unwrap();
            let short_len = if my_body_text.len() > 128 {
                128
            } else {
                my_body_text.len()
            };
            body_text_preview = my_body_text[..short_len].to_string();
            body_text_preview.push_str("...");
        }
        // This shit looks dumb af but okay.
        let save_db_cmd_1 = redis::Cmd::hset_multiple(format!("mail:{}", id), &[
            ("has_body_text", is_body_text),
            ("has_body_html", is_body_html),
        ]);
        let save_db_cmd_2 =
            redis::Cmd::hset(format!("mail:{}", id), "attachments", attachments_len);
        let save_db_cmd_3 = redis::Cmd::hset(
            format!("mail:{}", id),
            "timestamp",
            timestamp.to_timestamp(),
        );
        let save_db_cmd_4 = redis::Cmd::hset_multiple(format!("mail:{}", id), &[
            ("subject", subject),
            ("sender", sender),
            ("body_text_preview", body_text_preview),
        ]);
        let con_1 = &mut self.db.clone();
        let con_2 = &mut self.db.clone();
        let con_3 = &mut self.db.clone();
        let con_4 = &mut self.db.clone();
        match tokio::try_join!(
            save_db_cmd_1.exec_async(con_1),
            save_db_cmd_2.exec_async(con_2),
            save_db_cmd_3.exec_async(con_3),
            save_db_cmd_4.exec_async(con_4),
        ) {
            Ok(_) => {
                trace!("Saved mail to database: {}", id);
            }
            Err(e) => {
                error!("Failed to save mail to database: {}", e);
                return;
            }
        }
        // Save users
        for username in users {
            let save_db_cmd_1 = redis::Cmd::lpush(format!("user:{}", username), format!("mail:{}", id));
            let con_1 = &mut self.db.clone();
            match tokio::try_join!(
                save_db_cmd_1.exec_async(con_1),
            ) {
                Ok(_) => {
                    trace!("Saved user to database: {}", id);
                }
                Err(e) => {
                    error!("Failed to save user to database: {}", e);
                    return;
                }
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
