use axum::{response::IntoResponse, routing::get};
use tower_http::services::ServeDir;
use anyhow::Result;

#[derive(askama_axum::Template)]
#[template(path = "posts.html")]
struct Posts {
    signed_in: bool,
}

#[derive(askama_axum::Template)]
#[template(path = "upload.html")]
struct Upload {
    signed_in: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = axum::Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .route("/posts", get(posts))
        .route("/upload", get(upload));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5830").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn posts() -> impl IntoResponse {
    Posts { signed_in: false, }
}

async fn upload() -> impl IntoResponse {
    Upload { signed_in: false, }
}