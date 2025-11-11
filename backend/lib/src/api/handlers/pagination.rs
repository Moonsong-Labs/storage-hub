use axum::{
    extract::{rejection::QueryRejection, FromRequestParts, Query},
    http::request::Parts,
};
use serde::Deserialize;

use crate::constants::database::{DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT};

// NOTE: we use i64 because the db uses i64
/// Resolved pagination parameters for the given endpoint
///
/// The query parameters used for the requests are the ones described in [`PaginationQuery`]
///
/// Values:
/// * `limit`: the maximum amount of items to respond with
/// * `offset`: the number of items to skip for this request
#[derive(Debug)]
pub struct Pagination {
    pub limit: i64,
    pub offset: i64,
}

impl From<PaginationQuery> for Pagination {
    fn from(value: PaginationQuery) -> Self {
        let limit = value.limit.unwrap_or(DEFAULT_PAGE_LIMIT).clamp(
            0,
            MAX_PAGE_LIMIT
                .try_into()
                .expect("MAX_PAGE_LIMIT to be representable as i64"),
        );

        let offset = limit * value.page.unwrap_or(0);

        Self { offset, limit }
    }
}

// NOTE: we use i64 because the db uses i64
/// Pagination query parameters for the given endpoint
///
/// Parameters:
/// * `limit`: the maximum amount of items to respond with (defaults to 0, capped at [`MAX_PAGE_LIMIT`])
/// * `page`: the number of pages to skip for this request (defaults to 0), the number of elements in the page is equal to `limit`
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = QueryRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let query = Query::<PaginationQuery>::from_request_parts(parts, state).await?;

        Ok(query.0.into())
    }
}
