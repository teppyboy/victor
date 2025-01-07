use minismtp::server::SmtpServer;
use std::{env, path::Path, sync::Arc, time::Duration};
use tokio::{sync::Mutex, task};
use tracing::{error, info, trace, warn, debug};

mod config;
mod mail;
mod logging;
mod structs;

#[tokio::main]
async fn main() {
    let user_config: config::Config;
    if Path::new("./config.toml").exists() {
        user_config = config::Config::load("./config.toml");
    } else {
        user_config = config::Config::new();
        println!("Config file not found. Creating a new one...");
        user_config.save("./config.toml");
    }
    // Setup logging
    let level_str = user_config.log.level.clone();
    let log_level = env::var("LOG_LEVEL").unwrap_or(level_str);
    let log_file_name: Option<&str> = match &user_config.log.file.enabled {
        true => Some(&user_config.log.file.path),
        false => None,
    };
    logging::setup(&log_level, log_file_name).expect("Failed to setup logging.");
    if !user_config.smtp.enabled {
        warn!("SMTP server is disabled, but this is a SMTP server binary. Exiting...");
        return;
    }
    // Check if the features are enabled
    if !user_config.smtp.features.attachments {
        info!("Attachments are disabled, mail server will not save attachments.");
    } else {
        warn!("Attachments currently DOES NOT SAVE PROPERLY, USE AT YOUR OWN RISK.");
    }
    if !user_config.smtp.features.text_body {
        warn!("Plain text body is disabled, mail server will not save text body.");
    }
    if !user_config.smtp.features.html_body {
        warn!("HTML body is disabled, mail server will not save HTML body.");
    }
    // Setup SMTP server
    let host = user_config.smtp.host.clone();
    let port = user_config.smtp.port.clone();
    let domain = user_config.smtp.domain.clone();
    info!(
        "Starting server on '{}:{}' with domain '{}'",
        host, port, domain
    );
    let server = SmtpServer::new(
        host,
        port,
        domain,
        Some(Duration::from_secs(10)),
        None,
        None,
        None,
    );
    let listening_server = Arc::new(Mutex::new(server.start().await.unwrap()));
    debug!("Starting mail receiver task");
    let receiver_mutex = listening_server.clone();
    let receiver_handle = tokio::task::spawn(async move {
        let mail = mail::Mail::new(user_config.smtp.features);
        loop {
            let smtp_mail = receiver_mutex.lock().await.mail_rx.recv().await.unwrap();
            // I want to kms for having to clone the mail struct every time we receive a mail
            task::spawn(mail.clone().handle(smtp_mail));
        }
    });
    debug!("Mail receiver task started");
    info!("Server started");
    info!("Press Ctrl+C to stop the server");
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for event");
    info!("Stopping server");
    receiver_handle.abort();
    let close_mutex = listening_server.clone();
    close_mutex.lock().await.mail_rx.close();
    info!("Server stopped");
}
