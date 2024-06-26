use std::{fs, sync::Arc};
use askama_axum::IntoResponse;
use auth::OptionalAuthentication;
use axum::routing::get;
use tower_http::services::ServeDir;

mod posts;
mod auth;
mod config;
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    let conf_path = std::env::var("MINIBOORU_CONFIG_PATH").unwrap_or(String::from("./minibooru.conf"));
    let config: Arc<config::Config> = Arc::new(toml::from_str(&fs::read_to_string(conf_path)?)?);

    let db = sqlx::PgPool::connect(&config.network.database).await?;
    sqlx::migrate!().run(&db).await?;

    let app = axum::Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .route("/", get(index))
        .merge(posts::routes())
        .merge(auth::routes())
        .with_state(State {
            config: Arc::clone(&config),
            db,
        });

    let listener = tokio::net::TcpListener::bind(&config.network.bind).await?;
    log::info!("Serving on http://{}", config.network.bind);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(
    OptionalAuthentication(auth): OptionalAuthentication,
) -> impl IntoResponse {
    Index { 
        signed_in: auth.is_some(),
        post_count: 0,
        data_size: String::from("0 bytes"),
    }
}