use axum::{async_trait, extract::{FromRef, FromRequestParts}, http::request::Parts};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use uuid::Uuid;

use crate::{error::Error, traits::TransposeValues};

pub struct Settings {
    
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

        Ok(Self { db: state.db, group_id, id })
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Settings
where S: Send + Sync
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);

        Ok(Self { })
    }
}
