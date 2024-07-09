use anyhow::Context;
use axum::{async_trait, extract::{self, FromRef, FromRequestParts}, http::request::Parts};

#[derive(Default)]
pub struct Query {
    tags: Vec<String>,
    sort: Sort,
}

#[derive(Default)]
pub enum Sort {
    #[default]
    Date,
    Score,
}

#[async_trait]
impl<S> FromRequestParts<S> for Query
where
    crate::State: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = crate::Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // TODO: this might be worthy of an .expect() because i'm not sure how it'd ever fail
        let query_string: String = extract::Query::from_request_parts(parts, state)
                .await
                .context("parsing query string")?
                .0;
        let query_parts = query_string.split(' ').collect::<Vec<&str>>();

        let mut result = Query::default();

        for part in query_parts {
            if let Some((lhs, rhs)) = part.split_once(':') {
                match lhs {
                    "sort" => result.sort = Sort::try_from(rhs)?,
                    _ => return Err(crate::Error::Query(format!("Query category '{lhs}' does not exist"))),
                }
            } else {
                result.tags.push(part.to_string());
            }
        }

        Ok(result)
    }
}

impl TryFrom<&str> for Sort {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Sort, Self::Error> {
        match value {
            "date" => Ok(Self::Date),
            "score" => Ok(Self::Score),
            _ => Err(Self::Error::Query(format!("Invalid sort '{value}'"))),
        }
    }
}