use std::time::Duration;

use diesel::ConnectionError;
use diesel_async::{
    pooled_connection::{
        bb8::{Pool, PooledConnection},
        AsyncDieselConnectionManager,
    },
    AsyncPgConnection, RunQueryDsl,
};
use thiserror::Error;

pub mod models;
pub mod schema;
pub mod types;

pub use types::{OnchainBspId, OnchainMspId};

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConnection<'a> = PooledConnection<'a, AsyncPgConnection>;

pub type AsyncPgPool = Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;
pub type AsyncPgPooled = PooledConnection<'static, AsyncDieselConnectionManager<AsyncPgConnection>>;

#[derive(Error, Debug)]
pub enum DbSetupError {
    #[error("Failed to connect to the database: {0}")]
    ConnectionError(#[from] ConnectionError),
}

pub async fn setup_db_pool(database_url: String) -> Result<DbPool, DbSetupError> {
    let mgr = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

    let pool = Pool::builder()
        .max_size(16)
        .connection_timeout(Duration::from_secs(15))
        .idle_timeout(Some(Duration::from_secs(300)))
        .max_lifetime(Some(Duration::from_secs(3600)))
        .min_idle(Some(4))
        .build(mgr)
        .await
        .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

    // Health-check immediately (surface PG/libpq errors now, not later):
    {
        let mut conn = pool
            .get()
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?; // Obeys connection_timeout above
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
    }

    Ok(pool)
}
