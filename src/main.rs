#![feature(try_blocks, result_flattening)]
extern crate ffmpeg_next as ffmpeg;
use std::{fs, sync::Arc};
use askama_axum::IntoResponse;
use auth::Authentication;
use axum::{extract::DefaultBodyLimit, routing::get};
use tower_http::services::ServeDir;

mod posts;
mod auth;
mod config;
mod query;
mod error;
use error::{Result, Error};

#[derive(Clone)]
struct State {
    config: Arc<config::Config>,
    db: sqlx::PgPool,
}

#[derive(askama_axum::Template)]
#[template(path = "index.html")]
struct Index {
    signed_in: bool,
    post_count: u32,
    data_size: String,
}

const UNDERSTOOD_MIMES: &[&str] = &[
    "image/apng",
    "image/avif",
    "image/gif",
    "image/jpeg",
    "image/png",
    "image/webp",
    "image/bmp",
    "image/tiff",
    "video/webm",
    "video/ogg",
    "video/mp4",
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    ffmpeg::init()?;
    let conf_path = std::env::var("MINIBOORU_CONFIG_PATH").unwrap_or(String::from("./minibooru.conf"));
    let config: Arc<config::Config> = Arc::new(toml::from_str(&fs::read_to_string(conf_path)?)?);

    let db = sqlx::PgPool::connect(&config.network.database).await?;
    sqlx::migrate!().run(&db).await?;

    let mut app = axum::Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/static/thumb", ServeDir::new(config.data.thumbnails()))
        .nest_service("/static/media", ServeDir::new(config.data.media()))
        .route("/", get(index))
        .merge(posts::routes())
        .merge(auth::routes())
        .layer(DefaultBodyLimit::disable())
        .with_state(State {
            config: Arc::clone(&config),
            db,
        });

    log::debug!("FFmpeg build information: {}", ffmpeg::codec::configuration());

    let listener = tokio::net::TcpListener::bind(&config.network.bind).await?;
    log::info!("Serving on http://{}", config.network.bind);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(
    auth: Authentication,
) -> impl IntoResponse {
    Index { 
        signed_in: auth.signed_in(),
        post_count: 0,
        data_size: String::from("0 bytes"),
    }
}