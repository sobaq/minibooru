use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub network: Network,
    pub accounts: Accounts,
}

#[derive(Deserialize)]
pub struct Network {
    pub bind: std::net::SocketAddr,
    pub database: String,
}

#[derive(Deserialize)]
pub struct Accounts {
    #[serde(with = "serde_regex", rename = "username-regex")]
    pub username_regex: regex::Regex,
    #[serde(with = "serde_regex", rename = "password-regex")]
    pub password_regex: regex::Regex,
}