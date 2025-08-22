use diesel::ConnectionError;
use diesel_async::{
    pooled_connection::{
        bb8::{Pool, PooledConnection},
        AsyncDieselConnectionManager,
    },
    AsyncPgConnection,
};
use thiserror::Error;

pub mod models;
pub mod schema;
pub mod types;

pub use types::{OnchainBspId, OnchainMspId};

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConnection<'a> = PooledConnection<'a, AsyncPgConnection>;

#[derive(Error, Debug)]
pub enum DbSetupError {
    #[error("Failed to connect to the database: {0}")]
    ConnectionError(#[from] ConnectionError),
}

pub async fn setup_db_pool(database_url: String) -> Result<DbPool, DbSetupError> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(config)
        .await
        .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

    Ok(pool)
}
