use anyhow::Context;
use axum::{async_trait, extract::{self, FromRef, FromRequestParts}, http::request::Parts};

#[derive(serde::Deserialize)]
struct RawParams {
    #[serde(default)]
    query: String,
}

#[derive(Default, Debug)]
pub struct Query {
    tags: Vec<String>,
    sort: Sort,
}

#[derive(Default, Debug)]
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

    /// Failably parses the `query` query parameter without rejecting the requests
    /// so it can be reported elsewhere.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Query, Self::Rejection> {
        // TODO: this might be worthy of an .expect() because i'm not sure how it'd ever fail
        let query_params: RawParams = extract::Query::from_request_parts(parts, state)
                .await
                .context("parsing query string")?
                .0;
        let query_string = query_params.query.trim();
        let mut result = Query::default();

        if query_string.is_empty() {
            return Ok(result);
        }

        let query_parts = query_string.split(' ').collect::<Vec<&str>>();
        for part in query_parts {
            if let Some((lhs, rhs)) = part.split_once(':') {
                match lhs {
                    "sort" => result.sort = Sort::try_from(rhs)?,
                    _ => continue,
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