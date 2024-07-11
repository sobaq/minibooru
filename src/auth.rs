use argon2::{password_hash::{rand_core::OsRng, SaltString}, PasswordHasher, PasswordVerifier};
use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, routing::{get, post}, Form, Router};
use axum_extra::extract::{cookie::{Cookie, SameSite}, CookieJar};
use sqlx::types::Uuid;
use axum::{async_trait, extract::{FromRef, FromRequestParts}, http::request::Parts};

use crate::traits::TransposeValues;
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
pub struct Authentication {
    pub db: sqlx::PgPool,
    pub group_id: Option<i32>,
    pub id: Option<Uuid>,
}

pub struct Permission(pub Operation, pub Resource);

#[derive(sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Operation {
    Read,
    Modify,
    Delete,
    Create,
}

#[derive(sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Resource {
    Posts,
    Wiki,
}

impl Authentication {
    pub fn signed_in(&self) -> bool {
        self.id.is_some()
    }

    pub async fn has(&self, permission: Permission) -> crate::Result<bool> {
        Ok(sqlx::query_scalar("
            SELECT EXISTS (
                SELECT 1 AS result
                FROM groups
                WHERE id = $1
                AND superuser = true
            ) OR EXISTS (
                SELECT 1 AS result
                FROM permissions
                WHERE group_id IS NOT DISTINCT FROM $1
                AND operation = $2
                AND resource = $3
            );
        ")  .bind(self.group_id)
            .bind(permission.0)
            .bind(permission.1)
            .fetch_one(&self.db)
            .await?)
    }
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

#[async_trait]
impl<S> FromRequestParts<S> for Authentication
where
    crate::State: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = crate::State::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);

        let token = jar.get("session")
            .map(Cookie::value).map(Uuid::parse_str)
            .transpose().ok().flatten();

        let (id, group_id): (Option<Uuid>, Option<i32>) = sqlx::query_scalar("
            WITH user_id AS (
                SELECT user_id AS id
                FROM sessions
                WHERE token = $1
            ), group_id AS (
                SELECT group_id AS id
                FROM users, user_id
                WHERE users.id = user_id.id
            )
            SELECT (user_id.id, group_id.id)
            FROM user_id, group_id
        ")  .bind(token)
            .fetch_optional(&state.db)
            .await?
            .transpose_values();

        Ok(Authentication { db: state.db, group_id, id })
    }
}
