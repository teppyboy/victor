use minismtp::server::SmtpServer;
use std::{env, path::Path, sync::Arc, time::Duration};
use tokio::{sync::Mutex, task};
use tracing::{error, info, trace};

mod config;
mod handler;
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
        info!("SMTP server is disabled, goodbye.");
        return;
    }
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
    let receiver_mutex = listening_server.clone();
    let receiver_handle = tokio::task::spawn(async move {
        let mail_handler = handler::MailHandler::new(user_config.smtp.features);
        loop {
            let mail = receiver_mutex.lock().await.mail_rx.recv().await.unwrap();
            task::spawn(mail_handler.clone().handle(mail));
        }
    });
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
