use std::path::PathBuf;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub network: Network,
    pub accounts: Accounts,
    pub data: Data,
}

#[derive(Deserialize)]
pub struct Network {
    pub bind: std::net::SocketAddr,
    pub database: String,
}

#[derive(Deserialize)]
pub struct Data {
    media: std::path::PathBuf,
    pub thumbnails: DataThumbnails,
}

#[derive(Deserialize)]
pub struct DataThumbnails {
    pub resolution: u32,
}

#[derive(Deserialize)]
pub struct Accounts {
    #[serde(with = "serde_regex", rename = "username-regex")]
    pub username_regex: regex::Regex,
    #[serde(with = "serde_regex", rename = "password-regex")]
    pub password_regex: regex::Regex,
    #[serde(rename = "initial-superuser-password")]
    pub initial_superuser_password: String,
}

impl Data {
    /// Returns the root path for storing original-quality media
    pub fn media(&self) -> PathBuf {
        self.media.join("media")
    }
    
    /// Returns the root path for storing thumbnails
    pub fn thumbnails(&self) -> PathBuf {
        self.media.join("thumb")
    }
}