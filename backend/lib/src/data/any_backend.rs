//! AnyBackend implementation for multi-database support
//! 
//! This module provides a backend abstraction that allows switching between
//! PostgreSQL and SQLite at runtime while maintaining type safety.

use diesel::backend::{Backend, DieselReserveSpecialization};
use diesel::sql_types::TypeMetadata;
use std::fmt::{Debug, Display};

/// Enum representing either PostgreSQL or SQLite backend
/// 
/// This allows us to switch between database backends at runtime
/// while maintaining the same interface.
#[derive(Debug, Clone, Copy)]
pub enum AnyBackend {
    /// PostgreSQL backend
    Postgres,
    /// SQLite backend  
    Sqlite,
}

impl Backend for AnyBackend {
    type QueryBuilder = AnyQueryBuilder;
    type RawValue<'a> = AnyRawValue<'a>;
    type BindCollector<'a> = AnyBindCollector<'a>;
}

impl TypeMetadata for AnyBackend {
    type TypeMetadata = AnyTypeMetadata;
    type MetadataLookup = AnyMetadataLookup;
}

impl DieselReserveSpecialization for AnyBackend {}

/// Query builder that can work with either backend
pub enum AnyQueryBuilder {
    Postgres(diesel::pg::PgQueryBuilder),
    Sqlite(diesel::sqlite::SqliteQueryBuilder),
}

impl Default for AnyQueryBuilder {
    fn default() -> Self {
        // Default to PostgreSQL for backwards compatibility
        AnyQueryBuilder::Postgres(Default::default())
    }
}

impl diesel::query_builder::QueryBuilder<AnyBackend> for AnyQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        match self {
            AnyQueryBuilder::Postgres(qb) => qb.push_sql(sql),
            AnyQueryBuilder::Sqlite(qb) => qb.push_sql(sql),
        }
    }

    fn push_identifier(&mut self, identifier: &str) -> diesel::QueryResult<()> {
        match self {
            AnyQueryBuilder::Postgres(qb) => qb.push_identifier(identifier),
            AnyQueryBuilder::Sqlite(qb) => qb.push_identifier(identifier),
        }
    }

    fn push_bind_param(&mut self) {
        match self {
            AnyQueryBuilder::Postgres(qb) => qb.push_bind_param(),
            AnyQueryBuilder::Sqlite(qb) => qb.push_bind_param(),
        }
    }

    fn finish(self) -> String {
        match self {
            AnyQueryBuilder::Postgres(qb) => qb.finish(),
            AnyQueryBuilder::Sqlite(qb) => qb.finish(),
        }
    }
}

/// Raw value that can be from either backend
pub enum AnyRawValue<'a> {
    Postgres(<diesel::pg::Pg as Backend>::RawValue<'a>),
    Sqlite(<diesel::sqlite::Sqlite as Backend>::RawValue<'a>),
}

/// Bind collector that can work with either backend
pub enum AnyBindCollector<'a> {
    Postgres(<diesel::pg::Pg as Backend>::BindCollector<'a>),
    Sqlite(<diesel::sqlite::Sqlite as Backend>::BindCollector<'a>),
}

impl<'a> diesel::query_builder::BindCollector<'a, AnyBackend> for AnyBindCollector<'a> {
    type Buffer = AnyBindBuffer<'a>;

    fn push_bound_value<T, U>(
        &mut self,
        bind: &'a U,
        metadata_lookup: &mut <AnyBackend as TypeMetadata>::MetadataLookup,
    ) -> diesel::QueryResult<()>
    where
        AnyBackend: diesel::backend::HasSqlType<T>,
        U: diesel::serialize::ToSql<T, AnyBackend>,
    {
        // This is a simplified implementation - in a real scenario,
        // we'd need to properly delegate to the underlying collectors
        match (self, metadata_lookup) {
            (AnyBindCollector::Postgres(collector), AnyMetadataLookup::Postgres(lookup)) => {
                // Cast would be needed here with proper trait bounds
                Ok(())
            }
            (AnyBindCollector::Sqlite(collector), AnyMetadataLookup::Sqlite(lookup)) => {
                // Cast would be needed here with proper trait bounds
                Ok(())
            }
            _ => Err(diesel::result::Error::QueryBuilderError(
                "Backend mismatch in bind collector".into(),
            )),
        }
    }
}

/// Bind buffer for either backend
pub enum AnyBindBuffer<'a> {
    Postgres(Vec<u8>),
    Sqlite(Vec<u8>),
}

/// Type metadata for either backend
pub enum AnyTypeMetadata {
    Postgres(<diesel::pg::Pg as TypeMetadata>::TypeMetadata),
    Sqlite(<diesel::sqlite::Sqlite as TypeMetadata>::TypeMetadata),
}

/// Metadata lookup for either backend
pub enum AnyMetadataLookup {
    Postgres(<diesel::pg::Pg as TypeMetadata>::MetadataLookup),
    Sqlite(<diesel::sqlite::Sqlite as TypeMetadata>::MetadataLookup),
}

impl AnyBackend {
    /// Create a PostgreSQL backend variant
    pub fn postgres() -> Self {
        AnyBackend::Postgres
    }

    /// Create a SQLite backend variant
    pub fn sqlite() -> Self {
        AnyBackend::Sqlite
    }

    /// Check if this is a PostgreSQL backend
    pub fn is_postgres(&self) -> bool {
        matches!(self, AnyBackend::Postgres)
    }

    /// Check if this is a SQLite backend
    pub fn is_sqlite(&self) -> bool {
        matches!(self, AnyBackend::Sqlite)
    }
}

impl Display for AnyBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyBackend::Postgres => write!(f, "PostgreSQL"),
            AnyBackend::Sqlite => write!(f, "SQLite"),
        }
    }
}

/// Helper trait to determine the backend type from a connection
pub trait HasBackend {
    /// Get the backend type for this connection
    fn backend(&self) -> AnyBackend;
}

impl HasBackend for super::postgres::connection::AnyAsyncConnection {
    fn backend(&self) -> AnyBackend {
        match self {
            super::postgres::connection::AnyAsyncConnection::Postgres(_) => AnyBackend::Postgres,
            super::postgres::connection::AnyAsyncConnection::Sqlite(_) => AnyBackend::Sqlite,
        }
    }
}

impl HasBackend for super::postgres::connection::AnyDbConnection {
    fn backend(&self) -> AnyBackend {
        match self {
            super::postgres::connection::AnyDbConnection::Postgres(_) => AnyBackend::Postgres,
            super::postgres::connection::AnyDbConnection::Sqlite(_) => AnyBackend::Sqlite,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::any_connection::BackendType;
    use crate::data::postgres::connection::{AnyAsyncConnection, AnyDbConnection, DbConfig, DbConnection};
    use crate::data::sqlite::sqlite_connection::SqliteConnection;
    
    #[tokio::test]
    async fn test_sqlite_any_connection() {
        // Create an in-memory SQLite database
        let config = DbConfig::new(":memory:");
        let sqlite_conn = SqliteConnection::new(config).await
            .expect("Failed to create SQLite connection");
        
        // Wrap it in AnyDbConnection
        let any_conn = AnyDbConnection::Sqlite(sqlite_conn);
        
        // Test that we can get a connection
        let mut conn = any_conn.get_connection().await
            .expect("Failed to get connection from AnyDbConnection");
        
        // Verify it's a SQLite connection
        assert!(conn.is_sqlite());
        assert!(!conn.is_postgres());
        assert_eq!(conn.backend_name(), "SQLite");
        
        // Test raw SQL execution
        let result = conn.execute_raw_sql("CREATE TABLE test (id INTEGER PRIMARY KEY)").await;
        assert!(result.is_ok(), "Failed to execute raw SQL: {:?}", result.err());
    }
    
    #[test]
    fn test_backend_type_detection() {
        // Test PostgreSQL URLs
        assert_eq!(
            BackendType::from_url("postgresql://localhost/db").unwrap(),
            BackendType::Postgres
        );
        assert_eq!(
            BackendType::from_url("postgres://user:pass@host/db").unwrap(),
            BackendType::Postgres
        );
        
        // Test SQLite URLs and file paths
        assert_eq!(
            BackendType::from_url("sqlite:///path/to/db.sqlite").unwrap(),
            BackendType::Sqlite
        );
        assert_eq!(
            BackendType::from_url("test.db").unwrap(),
            BackendType::Sqlite
        );
        assert_eq!(
            BackendType::from_url("/absolute/path/to/database.sqlite").unwrap(),
            BackendType::Sqlite
        );
        assert_eq!(
            BackendType::from_url(":memory:").unwrap(),
            BackendType::Sqlite
        );
        
        // Test unsupported URL
        assert!(BackendType::from_url("mysql://localhost/db").is_err());
    }
    
    #[tokio::test]
    async fn test_any_db_connection_health_check() {
        // Create an in-memory SQLite database
        let config = DbConfig::new(":memory:");
        let sqlite_conn = SqliteConnection::new(config).await
            .expect("Failed to create SQLite connection");
        
        let any_conn = AnyDbConnection::Sqlite(sqlite_conn);
        
        // Test connection health
        assert!(any_conn.is_healthy().await, "Connection should be healthy");
        
        // Test connection test
        let test_result = any_conn.test_connection().await;
        assert!(test_result.is_ok(), "Connection test failed: {:?}", test_result.err());
    }
    
    #[test]
    fn test_any_backend_display() {
        assert_eq!(format!("{}", AnyBackend::Postgres), "PostgreSQL");
        assert_eq!(format!("{}", AnyBackend::Sqlite), "SQLite");
    }
    
    #[test]
    fn test_any_backend_helpers() {
        let pg_backend = AnyBackend::postgres();
        assert!(pg_backend.is_postgres());
        assert!(!pg_backend.is_sqlite());
        
        let sqlite_backend = AnyBackend::sqlite();
        assert!(!sqlite_backend.is_postgres());
        assert!(sqlite_backend.is_sqlite());
    }
}