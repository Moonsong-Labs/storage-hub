//! PostgreSQL repository implementation.
//!
//! This module provides the production repository implementation using
//! PostgreSQL as the backing database through diesel-async.
//!
//! ## Key Components
//! - [`Repository`] - PostgreSQL implementation of StorageOperations
//!
//! ## Features
//! - Connection pooling through SmartPool
//! - Automatic test transactions in test mode
//! - Type-safe queries through diesel
//! - Comprehensive error handling

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[cfg(test)]
use shc_indexer_db::OnchainBspId;
use shc_indexer_db::{
    models::{Bsp, Bucket, File, Msp},
    schema::{bsp, bucket, file},
    OnchainMspId,
};

#[cfg(test)]
use crate::data::indexer_db::repository::IndexerOpsMut;
use crate::data::indexer_db::repository::{
    error::RepositoryResult, pool::SmartPool, BucketId, IndexerOps,
};

/// PostgreSQL repository implementation.
///
/// Provides all database operations using a connection pool
/// with automatic test transaction management.
pub struct Repository {
    pool: SmartPool,
}

impl Repository {
    /// Create a new Repository with the given database URL.
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Returns
    /// * `Result<Self, RepositoryError>` - Repository instance or error
    pub async fn new(database_url: &str) -> RepositoryResult<Self> {
        Ok(Self {
            pool: SmartPool::new(database_url).await?,
        })
    }
}

#[async_trait]
impl IndexerOps for Repository {
    // ============ BSP Read Operations ============
    async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>> {
        let mut conn = self.pool.get().await?;

        let results: Vec<Bsp> = bsp::table
            .order(bsp::id.asc())
            .limit(limit)
            .offset(offset)
            .load(&mut *conn)
            .await?;

        Ok(results)
    }

    // ============ MSP Read Operations ============
    async fn get_msp_by_onchain_id(&self, msp: &OnchainMspId) -> RepositoryResult<Msp> {
        let mut conn = self.pool.get().await?;

        Msp::get_by_onchain_msp_id(&mut conn, msp.clone())
            .await
            .map_err(Into::into)
    }

    // ============ Bucket Read Operations ============
    async fn get_bucket_by_onchain_id(&self, bid: BucketId<'_>) -> RepositoryResult<Bucket> {
        let mut conn = self.pool.get().await?;

        Bucket::get_by_onchain_bucket_id(&mut conn, bid.0.to_owned())
            .await
            .map_err(Into::into)
    }

    async fn get_buckets_by_user_and_msp(
        &self,
        msp: i64,
        account: &str,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<Bucket>> {
        let mut conn = self.pool.get().await?;

        // Same as Bucket::get_user_buckets_by_msp but with pagination
        let buckets = bucket::table
            .order(bucket::id.asc())
            .filter(bucket::account.eq(account))
            .filter(bucket::msp_id.eq(msp))
            .limit(limit)
            .offset(offset)
            .load(&mut conn)
            .await?;

        Ok(buckets)
    }

    async fn get_files_by_bucket(
        &self,
        bucket: i64,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<File>> {
        let mut conn = self.pool.get().await?;

        // Same as File::get_by_bucket_id but with pagination
        let files = file::table
            .filter(file::bucket_id.eq(bucket))
            .limit(limit)
            .offset(offset)
            .load(&mut conn)
            .await?;

        Ok(files)
    }
}

#[cfg(test)]
#[async_trait]
impl IndexerOpsMut for Repository {
    // ============ BSP Write Operations ============
    async fn delete_bsp(&self, onchain_bsp_id: &OnchainBspId) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;

        diesel::delete(bsp::table.filter(bsp::onchain_bsp_id.eq(onchain_bsp_id)))
            .execute(&mut *conn)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use testcontainers::{runners::AsyncRunner, ContainerAsync};
    use testcontainers_modules::postgres::Postgres;

    // Embed the migrations from the indexer-db crate
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../client/indexer-db/migrations");

    /// Helper to set up a PostgreSQL container with migrations and optional init SQL
    async fn setup_test_db(init_sql: Option<&str>) -> (ContainerAsync<Postgres>, String) {
        // Create the Postgres container with custom configuration
        let mut postgres = Postgres::default()
            .with_db_name("test_db")
            .with_user("test")
            .with_password("test123");

        // Add init SQL if provided (executed after migrations)
        if let Some(sql) = init_sql {
            // We need to run migrations first, then the init SQL
            // So we'll handle init SQL separately after migrations
        }

        // Start the container
        let container = postgres
            .start()
            .await
            .expect("Failed to start postgres container");

        // Get the connection URL
        let host = container.get_host().await.expect("Failed to get host");
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get port");
        let database_url = format!("postgres://test:test123@{}:{}/test_db", host, port);

        // Run migrations and init SQL
        {
            use diesel::prelude::*;
            let mut conn = diesel::PgConnection::establish(&database_url)
                .expect("Failed to connect for migrations");
            conn.run_pending_migrations(MIGRATIONS)
                .expect("Failed to run migrations");

            // Execute init SQL if provided (after migrations)
            if let Some(sql) = init_sql {
                // Parse SQL and execute each statement separately
                // We need to handle comments and multiple statements
                let mut current_statement = String::new();
                for line in sql.lines() {
                    let trimmed = line.trim();
                    // Skip comment lines
                    if trimmed.starts_with("--") || trimmed.is_empty() {
                        continue;
                    }
                    current_statement.push_str(line);
                    current_statement.push('\n');

                    // Execute when we hit a semicolon at the end of a line
                    if trimmed.ends_with(';') {
                        let statement = current_statement.trim();
                        if !statement.is_empty() {
                            diesel::RunQueryDsl::execute(diesel::sql_query(statement), &mut conn)
                                .expect("Failed to execute init SQL");
                        }
                        current_statement.clear();
                    }
                }
                // Execute any remaining statement
                let remaining = current_statement.trim();
                if !remaining.is_empty() {
                    diesel::RunQueryDsl::execute(diesel::sql_query(remaining), &mut conn)
                        .expect("Failed to execute init SQL");
                }
            }
        }

        (container, database_url)
    }

    #[tokio::test]
    async fn test_repo_read() {
        // Initialize container with some test data
        let init_sql = r#"
            INSERT INTO bsp (id, account, capacity, stake, onchain_bsp_id, merkle_root)
            VALUES 
                (1, '0x1234567890abcdef', 1000, 100, '0x0000000000000000000000000000000000000000000000000000000000000001', '\x0102'),
                (2, '0xabcdef1234567890', 2000, 200, '0x0000000000000000000000000000000000000000000000000000000000000002', '\x0304');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let bsps = repo.list_bsps(10, 0).await.expect("Failed to fetch BSPs");
        assert_eq!(bsps.len(), 2, "Should have 2 BSPs");
        use bigdecimal::BigDecimal;
        assert_eq!(bsps[0].capacity, BigDecimal::from(1000));
        assert_eq!(bsps[1].capacity, BigDecimal::from(2000));
    }

    #[tokio::test]
    async fn test_repo_write() {
        // Initialize container with test data
        let init_sql = r#"
            INSERT INTO bsp (id, account, capacity, stake, onchain_bsp_id, merkle_root)
            VALUES 
                (1, '0x1111111111111111', 1000, 100, '0x0000000000000000000000000000000000000000000000000000000000000001', '\x0102'),
                (2, '0x2222222222222222', 2000, 200, '0x0000000000000000000000000000000000000000000000000000000000000002', '\x0304');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let original_bsps = repo.list_bsps(10, 0).await.expect("Failed to fetch BSPs");
        assert_eq!(original_bsps.len(), 2, "Should start with 2 BSPs");

        let bsp_to_delete = &original_bsps[0];
        // Use the onchain_bsp_id field directly since that's what OnchainBspId represents
        let bsp_id = bsp_to_delete.onchain_bsp_id.clone();

        repo.delete_bsp(&bsp_id)
            .await
            .expect("Failed to delete BSP");

        let changed_bsps = repo
            .list_bsps(10, 0)
            .await
            .expect("Failed to fetch BSPs after delete");
        assert_eq!(changed_bsps.len(), 1, "Should have 1 BSP after deletion");
        assert_ne!(
            changed_bsps[0].id, bsp_to_delete.id,
            "Remaining BSP should be different"
        );
    }

    #[tokio::test]
    async fn test_get_msp_by_onchain_id() {
        // Initialize container with MSP test data
        let init_sql = r#"
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp1234567890ab', 5000, 'Fast storage', '0x0000000000000000000000000000000000000000000000000000000000000000');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test MSP retrieval by onchain ID
        let msp = repo
            .get_msp_by_onchain_id(&OnchainMspId::new(shp_types::Hash::from([0; 32])))
            .await
            .expect("Failed to fetch MSP");

        use bigdecimal::BigDecimal;
        assert_eq!(msp.capacity, BigDecimal::from(5000));
        assert_eq!(msp.value_prop, "Fast storage");
        assert_eq!(msp.account, "0xmsp1234567890ab");
    }

    #[tokio::test]
    async fn test_get_bucket_by_onchain_id() {
        // Initialize container with bucket test data (requires MSP first)
        let init_sql = r#"
            -- Insert MSP first (required for foreign key)
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp1234567890ab', 5000, 'Storage provider', '0x0000000000000000000000000000000000000000000000000000000000000001');

            -- Insert Bucket
            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES 
                (1, '0xuser123', 1, 'my-bucket', '0xbucket123', false, '\x0102'),
                (2, '0xuser456', 1, 'private-bucket', '0xbucket456', true, '\x0304');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test bucket retrieval by onchain ID
        let bucket = repo
            .get_bucket_by_onchain_id(BucketId(b"0xbucket123"))
            .await
            .expect("Failed to fetch bucket");

        assert_eq!(bucket.name, b"my-bucket");
        assert_eq!(bucket.private, false);
        assert_eq!(bucket.account, "0xuser123");
    }

    #[tokio::test]
    async fn test_get_files_by_bucket() {
        // Initialize container with file test data (requires MSP and bucket first)
        let init_sql = r#"
            -- Insert MSP first
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp1234567890ab', 5000, 'Storage provider', '0x0000000000000000000000000000000000000000000000000000000000000001');

            -- Insert Bucket
            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES 
                (1, '0xuser123', 1, 'test-bucket', '0xbucket123', false, '\x0102');

            -- Insert Files (account and onchain_bucket_id are required)
            INSERT INTO file (id, account, file_key, bucket_id, onchain_bucket_id, location, fingerprint, size, step)
            VALUES 
                (1, '0xuser123', 'file1.txt', 1, '0xbucket123', '/data/file1.txt', '\x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20', 1024, 1),
                (2, '0xuser123', 'file2.txt', 1, '0xbucket123', '/data/file2.txt', '\x2021222324252627282930313233343536373839404142434445464748495051', 2048, 1),
                (3, '0xuser123', 'file3.txt', 1, '0xbucket123', '/data/file3.txt', '\x5253545556575859606162636465666768697071727374757677787980818283', 512, 0);
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test file retrieval by bucket
        let files = repo
            .get_files_by_bucket(1, 10, 0)
            .await
            .expect("Failed to fetch files");

        assert_eq!(files.len(), 3, "Should have 3 files in bucket");
        assert_eq!(files[0].file_key, b"file1.txt");
        assert_eq!(files[0].size, 1024);
        assert_eq!(files[1].file_key, b"file2.txt");
        assert_eq!(files[1].size, 2048);
        assert_eq!(files[2].file_key, b"file3.txt");
        assert_eq!(files[2].size, 512);
    }

    #[tokio::test]
    async fn test_get_buckets_by_user_and_msp() {
        // Initialize container with multiple buckets for testing filtering
        let init_sql = r#"
            -- Insert MSPs
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp111', 5000, 'MSP One', '0x0000000000000000000000000000000000000000000000000000000000000001'),
                (2, '0xmsp222', 8000, 'MSP Two', '0x0000000000000000000000000000000000000000000000000000000000000002');

            -- Insert Buckets for different users and MSPs
            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES 
                (1, '0xuser123', 1, 'user123-bucket1', '0xb1', false, '\x0102'),
                (2, '0xuser123', 1, 'user123-bucket2', '0xb2', true, '\x0304'),
                (3, '0xuser123', 2, 'user123-bucket3', '0xb3', false, '\x0506'),
                (4, '0xuser456', 1, 'user456-bucket1', '0xb4', true, '\x0708');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test getting buckets for user123 and msp_id=1
        let buckets = repo
            .get_buckets_by_user_and_msp(1, "0xuser123", 10, 0)
            .await
            .expect("Failed to fetch buckets");

        assert_eq!(
            buckets.len(),
            2,
            "Should have 2 buckets for user123 and MSP 1"
        );
        assert_eq!(buckets[0].name, b"user123-bucket1");
        assert_eq!(buckets[1].name, b"user123-bucket2");

        // Test pagination
        let first_bucket = repo
            .get_buckets_by_user_and_msp(1, "0xuser123", 1, 0)
            .await
            .expect("Failed to fetch first bucket");
        assert_eq!(first_bucket.len(), 1);
        assert_eq!(first_bucket[0].name, b"user123-bucket1");

        let second_bucket = repo
            .get_buckets_by_user_and_msp(1, "0xuser123", 1, 1)
            .await
            .expect("Failed to fetch second bucket");
        assert_eq!(second_bucket.len(), 1);
        assert_eq!(second_bucket[0].name, b"user123-bucket2");
    }
}
