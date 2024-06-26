use askama_axum::IntoResponse;
use axum::{routing::get, Router};

use crate::auth::OptionalAuthentication;

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

pub fn routes() -> Router<crate::State> {
    Router::new()
        .route("/posts", get(posts))
        .route("/posts/upload", get(upload))

        // .route("/api/posts/upload", post(api_upload))
}

async fn posts(OptionalAuthentication(auth): OptionalAuthentication) -> impl IntoResponse {
    Posts { signed_in: auth.is_some(), }
}

async fn upload(OptionalAuthentication(auth): OptionalAuthentication) -> impl IntoResponse {
    Upload { signed_in: auth.is_some(), }
}

// async fn api_upload(
//     State(state): State<crate::State>,
// ) -> impl IntoResponse {

// }