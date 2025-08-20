//! Test utilities and helpers for backend testing.
//!
//! This module provides common test fixtures and helper functions
//! for testing the backend services and repository implementations.
//!
//! Note: This is scaffolding test infrastructure with basic coverage.
//! Missing test utilities for:
//! - Complex data relationships and constraints validation
//! - Performance and load testing fixtures
//! - Database migration and rollback scenarios
//! - Mock data generators with realistic distributions
//! - Test data cleanup and isolation helpers
//! - Concurrent test execution coordination
//! - Property-based testing generators

#[cfg(test)]
use std::str::FromStr;

#[cfg(test)]
use bigdecimal::BigDecimal;

#[cfg(test)]
use crate::constants::test::{
    accounts::*, bsp::*, buckets::*, file_metadata::*, merkle::*, msp::*,
};
#[cfg(test)]
use crate::repository::{MockRepository, NewBsp, NewBucket, NewFile, Repository};

/// Creates a test repository with a real database connection.
///
/// This function creates a repository connected to a test database that will
/// automatically rollback all changes after the test completes.
///
/// # Panics
/// Panics if the test database URL is not configured or connection fails.
#[cfg(test)]
pub async fn create_test_repository() -> Repository {
    use crate::constants::test::DEFAULT_TEST_DATABASE_URL;

    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| DEFAULT_TEST_DATABASE_URL.to_string());

    Repository::new(&database_url)
        .await
        .expect("Failed to create test repository")
}

/// Creates a mock repository for unit testing.
///
/// This returns an in-memory mock repository that doesn't require
/// any database connection.
#[cfg(test)]
pub fn create_mock_repository() -> MockRepository {
    MockRepository::new()
}

/// Creates a sample BSP for testing.
#[cfg(test)]
pub fn create_test_bsp(account_suffix: &str) -> NewBsp {
    NewBsp {
        account: format!("{}_{}", TEST_BSP_ACCOUNT_STR, account_suffix),
        capacity: BigDecimal::from_str(&DEFAULT_CAPACITY.to_string()).unwrap(),
        stake: BigDecimal::from_str(&DEFAULT_STAKE.to_string()).unwrap(),
        onchain_bsp_id: format!("{}{}", TEST_BSP_ONCHAIN_ID_PREFIX, account_suffix),
        merkle_root: BSP_MERKLE_ROOT.to_vec(),
    }
}

/// Creates a sample Bucket for testing.
#[cfg(test)]
pub fn create_test_bucket(account_suffix: &str, msp_id: Option<i64>) -> NewBucket {
    NewBucket {
        msp_id: msp_id.or(Some(DEFAULT_MSP_ID)),
        account: format!("{}_{}", TEST_USER_ACCOUNT_STR, account_suffix),
        onchain_bucket_id: TEST_ONCHAIN_BUCKET_ID.to_vec(),
        name: TEST_BUCKET_NAME_STR.to_vec(),
        collection_id: Some(format!("collection_{}", account_suffix)),
        private: false,
        merkle_root: BUCKET_MERKLE_ROOT.to_vec(),
    }
}

/// Creates a sample File for testing.
#[cfg(test)]
pub fn create_test_file(key_suffix: &str, bucket_id: i64) -> NewFile {
    NewFile {
        account: TEST_USER_ACCOUNT.to_vec(),
        file_key: format!(
            "{}_{}",
            std::str::from_utf8(TEST_FILE_KEY_STR).unwrap(),
            key_suffix
        )
        .into_bytes(),
        bucket_id,
        location: TEST_LOCATION_STR.to_vec(),
        fingerprint: TEST_FINGERPRINT.to_vec(),
        size: TEST_FILE_SIZE as i64,
        step: INITIAL_STEP as i32,
    }
}

/// Asserts that two BSPs are equal, ignoring timestamps and IDs.
#[cfg(test)]
pub fn assert_bsp_eq_ignore_timestamps(actual: &crate::repository::Bsp, expected: &NewBsp) {
    assert_eq!(actual.account, expected.account);
    assert_eq!(actual.capacity, expected.capacity);
    assert_eq!(actual.stake, expected.stake);
    assert_eq!(actual.onchain_bsp_id, expected.onchain_bsp_id);
    assert_eq!(actual.merkle_root, expected.merkle_root);
    // Note: multiaddresses are stored in a separate table (bsp_multiaddress)
}

/// Asserts that two Buckets are equal, ignoring timestamps and IDs.
#[cfg(test)]
pub fn assert_bucket_eq_ignore_timestamps(
    actual: &crate::repository::Bucket,
    expected: &NewBucket,
) {
    assert_eq!(actual.msp_id, expected.msp_id);
    assert_eq!(actual.account, expected.account);
    assert_eq!(actual.onchain_bucket_id, expected.onchain_bucket_id);
    assert_eq!(actual.name, expected.name);
    assert_eq!(actual.collection_id, expected.collection_id);
    assert_eq!(actual.private, expected.private);
    assert_eq!(actual.merkle_root, expected.merkle_root);
}

/// Asserts that two Files are equal, ignoring timestamps and IDs.
#[cfg(test)]
pub fn assert_file_eq_ignore_timestamps(actual: &crate::repository::File, expected: &NewFile) {
    assert_eq!(actual.account, expected.account);
    assert_eq!(actual.file_key, expected.file_key);
    assert_eq!(actual.bucket_id, expected.bucket_id);
    assert_eq!(actual.location, expected.location);
    assert_eq!(actual.fingerprint, expected.fingerprint);
    assert_eq!(actual.size, expected.size);
    assert_eq!(actual.step, expected.step);
}

/// Test fixture for setting up a repository with sample data.
#[cfg(test)]
pub struct TestFixture {
    pub bsp1_id: i64,
    pub bsp2_id: i64,
    pub bucket1_id: i64,
    pub bucket2_id: i64,
    pub file1_key: Vec<u8>,
    pub file2_key: Vec<u8>,
}

#[cfg(test)]
impl TestFixture {
    /// Creates a new test fixture with sample data in the repository.
    pub async fn setup(repo: &(impl crate::repository::IndexerOpsMut + Send + Sync)) -> Self {
        // Clear any existing data
        repo.clear_all().await;

        // Create BSPs
        let bsp1 = repo.create_bsp(create_test_bsp("1")).await.unwrap();
        let bsp2 = repo.create_bsp(create_test_bsp("2")).await.unwrap();

        // Create Buckets
        let bucket1 = repo
            .create_bucket(create_test_bucket("1", Some(DEFAULT_MSP_ID)))
            .await
            .unwrap();
        let bucket2 = repo
            .create_bucket(create_test_bucket("2", None))
            .await
            .unwrap();

        // Create Files
        let file1 = create_test_file("1", bucket1.id);
        let file1_key = file1.file_key.clone();
        repo.create_file(file1).await.unwrap();

        let file2 = create_test_file("2", bucket2.id);
        let file2_key = file2.file_key.clone();
        repo.create_file(file2).await.unwrap();

        TestFixture {
            bsp1_id: bsp1.id,
            bsp2_id: bsp2.id,
            bucket1_id: bucket1.id,
            bucket2_id: bucket2.id,
            file1_key,
            file2_key,
        }
    }
}
