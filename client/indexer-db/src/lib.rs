use std::env;

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

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConnection<'a> = PooledConnection<'a, AsyncPgConnection>;

#[derive(Error, Debug)]
pub enum DbSetupError {
    #[error("Failed to connect to the database: {0}")]
    ConnectionError(#[from] ConnectionError),
    #[error("Failed to read DATABASE_URL environment variable: {0}")]
    EnvVarError(#[from] env::VarError),
}

pub async fn setup_db_pool() -> Result<DbPool, DbSetupError> {
    let database_url = env::var("DATABASE_URL")?;

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(config)
        .await
        .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

    Ok(pool)
}
