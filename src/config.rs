use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileLog {
    pub enabled: bool,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Log {
    pub level: String,
    pub file: FileLog,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Features {
    pub attachments: bool,
    pub text_body: bool,
    pub html_body: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TLS {
    pub enabled: bool,
    pub cert_path: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SMTP {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub features: Features,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Database {
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub log: Log,
    pub database: Database,
    pub smtp: SMTP,
}

impl Config {
    pub fn new() -> Config {
        Config {
            log: Log {
                level: "info".to_string(),
                file: FileLog {
                    enabled: false,
                    path: "victor-%Y%m%d-%H%M%S.log".to_string(),
                },
            },
            // I don't know if this shit works or not tbh
            database: Database {
                url: "redis://localhost:6379".to_string(),
            },
            smtp: SMTP {
                enabled: true,
                host: "::".to_string(),
                port: 25,
                domain: "localhost".to_string(),
                features: Features {
                    attachments: false,
                    text_body: true,
                    html_body: true,
                },
            },
        }
    }
    pub fn save(&self, path: &str) {
        let toml = toml::to_string(&self).unwrap();
        fs::write(path, toml).expect("Failed to write config file");
    }
    pub fn load(path: &str) -> Config {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        let config: Config = toml::from_str(&content.as_str()).unwrap();
        return config;
    }
}
