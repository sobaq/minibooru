use argon2::{password_hash::{rand_core::OsRng, SaltString}, PasswordHasher, PasswordVerifier};
use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, routing::{get, post}, Form, Router};
use axum_extra::extract::{cookie::{Cookie, SameSite}, CookieJar};
use sqlx::types::Uuid;

use crate::{error::ResultExt, extractors::Authentication};

#[derive(askama_axum::Template)]
#[template(path = "auth.html")]
struct Auth {
    signed_in: bool,
    username_regex: regex::Regex,
    password_regex: regex::Regex,
}

#[derive(serde::Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

pub fn routes() -> Router<crate::State> {
    Router::new()
        .route("/auth", get(auth_page))

        .route("/api/auth/logout", post(logout))
        .route("/api/auth/login", post(login))
        .route("/api/auth/register", post(register))
}

async fn auth_page(
    auth: Authentication,
    State(state): State<crate::State>,
) -> impl IntoResponse {
    Auth {
        signed_in: auth.signed_in(),
        username_regex: state.config.accounts.username_regex.clone(),
        password_regex: state.config.accounts.password_regex.clone(),
    }
}

async fn logout(
    jar: CookieJar,
    State(state): State<crate::State>,
) -> (CookieJar, Redirect) {
    if let Some(Ok(session)) = jar.get("session").map(|c| Uuid::parse_str(c.value())) {
        _ = sqlx::query("
            DELETE FROM sessions WHERE token = $1;
        ").bind(session).execute(&state.db).await;

        // TODO: this doesn't remove the cookie for some reason
        (jar.remove("session"), Redirect::to("/"))
    } else {
        (jar, Redirect::to("/"))
    }
}

async fn login(
    jar: CookieJar,
    State(state): State<crate::State>,
    Form(desired): Form<Credentials>
) -> crate::Result<CookieJar> {
    let (user_id, correct_password): (Uuid, String) = sqlx::query_scalar("
        SELECT (id, password)
        FROM users
        WHERE username = $1; 
    ")  .bind(desired.username)
        .fetch_one(&state.db)
        .await
        .on_no_rows(crate::Error::Unauthorized)?;

    let password_ok = argon2::PasswordHash::new(&correct_password)
        .map(|correct| argon2::Argon2::default().verify_password(desired.password.as_bytes(), &correct))
        .flatten()
        .is_ok();

    if password_ok {
        add_sign_in_cookie(&state.db, user_id, jar).await
    } else {
        Err(crate::Error::Unauthorized)
    }
}

async fn register(
    jar: CookieJar,
    State(state): State<crate::State>,
    Form(desired): Form<Credentials>,
) -> crate::Result<CookieJar> {
    if !state.config.accounts.username_regex.is_match(&desired.username) {
        return Err(crate::Error::BadRequest(String::from("Username does not meet requirements")));
    }

    if !state.config.accounts.password_regex.is_match(&desired.password) {
        return Err(crate::Error::BadRequest(String::from("Password does not meet requirements")));
    }

    let password = kdf(&desired.password);
    let id: Uuid = sqlx::query_scalar("
        INSERT INTO users (username, password)
        VALUES ($1, $2)
        RETURNING id;
    ")  .bind(desired.username)
        .bind(&password)
        .fetch_one(&state.db)
        .await
        .on_constraint("username_unique", |_| crate::Error::Conflict(String::from("Username already taken")))?;

    add_sign_in_cookie(&state.db, id, jar).await
}

async fn add_sign_in_cookie(db: &sqlx::PgPool, user_id: Uuid, jar: CookieJar) -> crate::Result<CookieJar> {
    let token: Uuid = sqlx::query_scalar("
        INSERT INTO sessions (user_id)
        VALUES ($1)
        RETURNING token;
    ").bind(&user_id).fetch_one(db).await?;

    let cookie = Cookie::build(("session", token.as_hyphenated().to_string()))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .build();

    Ok(jar.add(cookie))
}

pub fn kdf(input: &str) -> String {
    let hasher = argon2::Argon2::default();
    hasher.hash_password(input.as_bytes(), &SaltString::generate(&mut OsRng)).unwrap().to_string()
}
