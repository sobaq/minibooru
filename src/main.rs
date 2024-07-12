#![feature(try_blocks, result_flattening)]
extern crate ffmpeg_next as ffmpeg;
use std::{fs, sync::Arc};
use askama_axum::IntoResponse;
use axum::{extract::{self, DefaultBodyLimit}, routing::get};
use extractors::Authentication;
use tower_http::services::ServeDir;

mod extractors;
mod traits;
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
    post_count: i64,
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

    let app = axum::Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/static/thumb", ServeDir::new(config.data.thumbnails()))
        .nest_service("/static/media", ServeDir::new(config.data.media()))
        
        .route("/", get(index))
        .route("/settings", get(settings))
        .merge(posts::routes())
        .merge(auth::routes())
        .layer(DefaultBodyLimit::disable())
        .with_state(State {
            config: Arc::clone(&config),
            db: db.clone(),
        });

    let account_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users;").fetch_one(&db).await?;
    if account_count == 0 {
        let password = auth::kdf(&config.accounts.initial_superuser_password);
        sqlx::query("
            WITH superuser_group_id AS (
                INSERT INTO groups (name, superuser)
                VALUES ('superusers', true)
                RETURNING id
            )
            INSERT INTO users (group_id, username, password)
            SELECT id, 'superuser', $1
            FROM superuser_group_id;
        ").bind(&password).execute(&db).await?;
        log::info!("Created 'superuser' account with initial password");
    }

    log::debug!("FFmpeg build information: {}", ffmpeg::codec::configuration());

    let listener = tokio::net::TcpListener::bind(&config.network.bind).await?;
    log::info!("Serving on http://{}", config.network.bind);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(
    extract::State(state): extract::State<crate::State>,
    auth: Authentication,
) -> crate::Result<impl IntoResponse> {
    let (post_count, data_size): (i64, i64) = sqlx::query_scalar("
        SELECT (COUNT(*)::BIGINT, COALESCE(SUM(posts.file_size), 0)::BIGINT)
        FROM posts
    ").fetch_one(&state.db).await?;
    let data_size = readable_file_size(data_size as _)?;

    Ok(Index {
        signed_in: auth.signed_in(),
        post_count,
        data_size,
    })
}

async fn settings(
) -> impl IntoResponse {
    "Imagine a settings page here"
}

fn readable_file_size(raw: u64) -> anyhow::Result<String> {
    let mut raw = raw as f64;
    for unit in &["", "Ki", "Mi", "Gi"] {
        if raw < 1024. {
            return Ok(format!("{raw:.1}\u{00A0}{unit}B"));
        }

        raw /= 1024.;
    }

    anyhow::bail!("Given byte count too large to compute a human readable file size")
}
