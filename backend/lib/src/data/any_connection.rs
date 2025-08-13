//! Backend-agnostic connection utilities
//! 
//! This module provides utilities for working with connections regardless
//! of the underlying database backend (PostgreSQL or SQLite).

use super::postgres::connection::{AnyDbConnection, DbConnectionError};

/// Backend type enumeration for runtime backend detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// PostgreSQL backend
    Postgres,
    /// SQLite backend
    Sqlite,
}

impl BackendType {
    /// Parse backend type from a database URL
    pub fn from_url(url: &str) -> Result<Self, String> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(BackendType::Postgres)
        } else if url.starts_with("sqlite://") || url.ends_with(".db") || url.ends_with(".sqlite") || url == ":memory:" {
            Ok(BackendType::Sqlite)
        } else {
            Err(format!("Unable to determine backend type from URL: {}", url))
        }
    }
}

/// Builder for creating AnyDbConnection based on configuration
pub struct AnyDbConnectionBuilder;

impl AnyDbConnectionBuilder {
    /// Create a new AnyDbConnection based on the database URL
    pub async fn from_url(url: &str) -> Result<AnyDbConnection, DbConnectionError> {
        let backend = BackendType::from_url(url)
            .map_err(|e| DbConnectionError::Config(e))?;
        
        match backend {
            BackendType::Postgres => {
                // Import the PgConnection type
                use super::postgres::pg_connection::PgConnection;
                use super::postgres::connection::DbConfig;
                
                let config = DbConfig::new(url);
                let pg_conn = PgConnection::new(config).await
                    .map_err(|e| DbConnectionError::Config(format!("Failed to create PostgreSQL connection: {}", e)))?;
                Ok(AnyDbConnection::Postgres(pg_conn))
            }
            BackendType::Sqlite => {
                // Import the SqliteConnection type
                use super::sqlite::sqlite_connection::SqliteConnection;
                use super::postgres::connection::DbConfig;
                
                let config = DbConfig::new(url);
                let sqlite_conn = SqliteConnection::new(config).await
                    .map_err(|e| DbConnectionError::Config(format!("Failed to create SQLite connection: {}", e)))?;
                Ok(AnyDbConnection::Sqlite(sqlite_conn))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_type_from_url() {
        assert_eq!(
            BackendType::from_url("postgresql://localhost/test").unwrap(),
            BackendType::Postgres
        );
        assert_eq!(
            BackendType::from_url("postgres://localhost/test").unwrap(),
            BackendType::Postgres
        );
        assert_eq!(
            BackendType::from_url("sqlite://test.db").unwrap(),
            BackendType::Sqlite
        );
        assert_eq!(
            BackendType::from_url("test.sqlite").unwrap(),
            BackendType::Sqlite
        );
        assert_eq!(
            BackendType::from_url("/path/to/database.db").unwrap(),
            BackendType::Sqlite
        );
        assert_eq!(
            BackendType::from_url(":memory:").unwrap(),
            BackendType::Sqlite
        );
        
        assert!(BackendType::from_url("mysql://localhost/test").is_err());
    }
}