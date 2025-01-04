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
pub struct SMTP {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub domain: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub log: Log,
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
            smtp: SMTP {
                enabled: true,
                host: "::".to_string(),
                port: 25,
                domain: "localhost".to_string(),
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
