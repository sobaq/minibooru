use argon2::{PasswordHasher, password_hash::{rand_core::OsRng, SaltString}};
use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, routing::{get, post}, Form, Router};
use axum_extra::extract::{cookie::{Cookie, SameSite}, CookieJar};
use sqlx::types::Uuid;
use axum::{async_trait, extract::{FromRef, FromRequestParts}, http::request::Parts};
use serde::Deserialize;

use crate::error::{Error, ResultExt};

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

/// Extract and validate a users session token, retrieving their user ID.
#[derive(Deserialize)]
pub struct Authentication {
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct OptionalAuthentication(pub Option<Authentication>);

pub fn routes() -> Router<crate::State> {
    Router::new()
        .route("/auth", get(auth))
        .route("/auth/logout", post(logout))

        .route("/api/register", post(register))
}

async fn auth(
    OptionalAuthentication(auth): OptionalAuthentication,
    State(state): State<crate::State>,
) -> impl IntoResponse {
    Auth {
        signed_in: auth.is_some(),
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

        (jar.remove("session"), Redirect::to("/"))
    } else {
        (jar, Redirect::to("/"))
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

    let token: Uuid = sqlx::query_scalar("
        INSERT INTO sessions (user_id)
        VALUES ($1)
        RETURNING token;
    ").bind(&id).fetch_one(&state.db).await?;

    let cookie = Cookie::build(("session", token.as_hyphenated().to_string()))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .build();
    
    Ok(jar.add(cookie))
}

fn kdf(input: &str) -> String {
    let hasher = argon2::Argon2::default();
    hasher.hash_password(input.as_bytes(), &SaltString::generate(&mut OsRng)).unwrap().to_string()
}

/// Returns None when the user is not signed in.
#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuthentication
where
    crate::State: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(cookie) = jar.get("session") else {
            return Ok(Self(None));
        };
        let token = Uuid::parse_str(cookie.value()).map_err(|_| crate::Error::Unauthorized)?;
        let state = crate::State::from_ref(state);

        let user_id: Option<Uuid> = sqlx::query_scalar("
            SELECT user_id FROM sessions WHERE token = $1;
        ")  .bind(token)
            .fetch_optional(&state.db)
            .await?;

        match user_id {
            None => Ok(Self(None)),
            Some(id) => Ok(Self(Some(Authentication {
                id,
            }))),
        }
    }
}

/// Fails with 403 UNAUTHORIZED when a session token does not exist.
#[async_trait]
impl<S> FromRequestParts<S> for Authentication
where
    crate::State: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = Uuid::parse_str(jar.get("session").ok_or(Error::Unauthorized)?.value())
            .map_err(|_| crate::Error::Unauthorized)?;
    
        let state = crate::State::from_ref(state);

        let id: Uuid = sqlx::query_scalar("
            SELECT user_id FROM sessions WHERE token = $1;
        ")  .bind(token)
            .fetch_one(&state.db)
            .await
            .on_no_rows(Error::Unauthorized)?;

        Ok(Self {
            id,
        })
    }
}
