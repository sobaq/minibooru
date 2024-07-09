use axum::{http::StatusCode, response::{IntoResponse, Response}};
use sqlx::error::DatabaseError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Content Too Large")]
    ContentTooLarge(String),
    #[error("Conflict")]
    Conflict(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Bad Request")]
    BadRequest(String),
    #[error("Unsupported Media Type")]
    UnsupportedMediaType(String),
    #[error("Not Found")]
    NotFound,

    #[error("Bad query: {0}")]
    Query(String),
    #[error("SQL query")]
    Sql(#[from] sqlx::Error),
    #[error("Internal Server Error")]
    Internal(#[from] anyhow::Error),
}

pub trait ResultExt<T> {
    fn on_constraint(
        self,
        name: &str,
        f: impl FnOnce(Box<dyn DatabaseError>) -> Error,
    ) -> Result<T>;

    fn on_no_rows(
        self,
        e: Error,
    ) -> Result<T>;
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::ContentTooLarge(content) =>
                (StatusCode::PAYLOAD_TOO_LARGE, content),
            Error::Conflict(conflict) =>
                (StatusCode::CONFLICT, conflict),
            Error::Unauthorized =>
                (StatusCode::UNAUTHORIZED, String::new()),
            Error::BadRequest(s) =>
                (StatusCode::BAD_REQUEST, s),
            Error::UnsupportedMediaType(ty) =>
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, ty),
            Error::NotFound =>
                (StatusCode::NOT_FOUND, String::new()),

            Error::Query(s) =>
                (StatusCode::BAD_REQUEST, s),
            Error::Internal(e) => {
                log::error!("Internal error: {e:?}");

                (StatusCode::INTERNAL_SERVER_ERROR, String::new())
            },
            Error::Sql(e) => {
                log::error!("SQL error: {e}");

                (StatusCode::INTERNAL_SERVER_ERROR, String::new())
            }
        }.into_response()
    }
}

// Can thiserror handle this for us?
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Internal(value.into())
    }
}

impl From<axum::extract::multipart::MultipartError> for Error {
    fn from(value: axum::extract::multipart::MultipartError) -> Self {
        Self::Internal(value.into())
    }
}

// impl From<image::ImageError> for Error {
//     fn from(value: image::ImageError) -> Self {
//         Self::Internal(value.into())
//     }
// }

impl From<ffmpeg_next::Error> for Error {
    fn from(value: ffmpeg_next::Error) -> Self {
        Self::Internal(value.into())
    }
}

impl<T, E> ResultExt<T> for Result<T, E>
where E: Into<Error>,
{
    fn on_constraint(
        self,
        name: &str,
        map_err: impl FnOnce(Box<dyn DatabaseError>) -> Error,
    ) -> Result<T, Error> {
        self.map_err(|e| match e.into() {
            Error::Sql(sqlx::Error::Database(dbe)) if dbe.constraint() == Some(name) => {
                map_err(dbe)
            }
            e => e,
        })
    }

    fn on_no_rows(
            self,
            e: Error,
        ) -> Result<T> {
        self.map_err(|er| match er.into() {
            Error::Sql(sqlx::Error::RowNotFound) => {
                e
            }
            er => er,
        })
    }
}
