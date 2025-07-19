//! Mock PostgreSQL client for testing
//!
//! This module provides a mock implementation of the PostgreSQL client that returns
//! realistic test data matching the shc-indexer-db models.

use async_trait::async_trait;
use chrono::NaiveDateTime;
use shc_indexer_db::models::{
    Bucket, BspFile, File, FileStorageRequestStep, Msp, PeerId as DbPeerId,
};
use std::sync::{Arc, Mutex};

use crate::{
    data::postgres::{PostgresClientTrait, PaginationParams},
    error::{Error, Result},
};

/// Mock data storage
#[derive(Debug, Default)]
struct MockData {
    files: Vec<File>,
    buckets: Vec<Bucket>,
    msps: Vec<Msp>,
    peer_ids: Vec<DbPeerId>,
    bsp_files: Vec<BspFile>,
}

/// Mock PostgreSQL client for testing
#[derive(Debug, Clone)]
pub struct MockPostgresClient {
    data: Arc<Mutex<MockData>>,
}

impl MockPostgresClient {
    /// Create a new mock PostgreSQL client with default test data
    pub fn new() -> Self {
        let mut data = MockData::default();
        
        // Create test MSPs
        data.msps.push(Msp {
            id: 1,
            onchain_msp_id: vec![1, 2, 3, 4],
            account: vec![10, 11, 12, 13],
            value_prop: vec![100, 101, 102],
            created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
        });

        // Create test buckets
        data.buckets.push(Bucket {
            id: 1,
            msp_id: Some(1),
            account: hex::encode(&[50, 51, 52, 53]), // Same as user account
            onchain_bucket_id: vec![30, 31, 32, 33],
            name: vec![110, 111, 112, 113],
            collection_id: None,
            private: false,
            created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            merkle_root: vec![40, 41, 42, 43],
        });

        // Create test peer IDs
        data.peer_ids.push(DbPeerId {
            id: 1,
            peer_id: vec![60, 61, 62, 63],
        });

        // Create test files
        data.files.push(File {
            id: 1,
            account: vec![50, 51, 52, 53], // Same as bucket user_id
            file_key: vec![70, 71, 72, 73],
            bucket_id: 1,
            location: vec![80, 81, 82, 83],
            fingerprint: vec![90, 91, 92, 93],
            size: 1024,
            step: FileStorageRequestStep::Stored as i32,
            created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
        });

        data.files.push(File {
            id: 2,
            account: vec![50, 51, 52, 53],
            file_key: vec![74, 75, 76, 77],
            bucket_id: 1,
            location: vec![84, 85, 86, 87],
            fingerprint: vec![94, 95, 96, 97],
            size: 2048,
            step: FileStorageRequestStep::Requested as i32,
            created_at: NaiveDateTime::from_timestamp_opt(1_700_001_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_001_000, 0).unwrap(),
        });

        Self {
            data: Arc::new(Mutex::new(data)),
        }
    }

    /// Add a test file to the mock data
    pub fn add_test_file(&self, file: File) {
        let mut data = self.data.lock().unwrap();
        data.files.push(file);
    }

    /// Add a test bucket to the mock data
    pub fn add_test_bucket(&self, bucket: Bucket) {
        let mut data = self.data.lock().unwrap();
        data.buckets.push(bucket);
    }

    /// Add a test MSP to the mock data
    pub fn add_test_msp(&self, msp: Msp) {
        let mut data = self.data.lock().unwrap();
        data.msps.push(msp);
    }

    /// Clear all mock data
    pub fn clear_data(&self) {
        let mut data = self.data.lock().unwrap();
        data.files.clear();
        data.buckets.clear();
        data.msps.clear();
        data.peer_ids.clear();
        data.bsp_files.clear();
    }
}

impl Default for MockPostgresClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PostgresClientTrait for MockPostgresClient {
    async fn test_connection(&self) -> Result<()> {
        // Always succeed for mock
        Ok(())
    }

    async fn get_file_by_key(&self, file_key: &[u8]) -> Result<File> {
        let data = self.data.lock().unwrap();
        data.files
            .iter()
            .find(|f| f.file_key == file_key)
            .cloned()
            .ok_or_else(|| Error::NotFound("File not found".to_string()))
    }

    async fn get_files_by_user(
        &self,
        user_account: &[u8],
        pagination: Option<PaginationParams>,
    ) -> Result<Vec<File>> {
        let data = self.data.lock().unwrap();
        let mut files: Vec<File> = data
            .files
            .iter()
            .filter(|f| f.account == user_account)
            .cloned()
            .collect();

        // Apply pagination
        if let Some(params) = pagination {
            let offset = params.offset.unwrap_or(0) as usize;
            let limit = params.limit.unwrap_or(20) as usize;
            files = files.into_iter().skip(offset).take(limit).collect();
        }

        Ok(files)
    }

    async fn get_files_by_user_and_msp(
        &self,
        user_account: &[u8],
        msp_id: i64,
        pagination: Option<PaginationParams>,
    ) -> Result<Vec<File>> {
        let data = self.data.lock().unwrap();
        
        // Get buckets for the MSP
        let bucket_ids: Vec<i64> = data
            .buckets
            .iter()
            .filter(|b| b.msp_id == msp_id)
            .map(|b| b.id)
            .collect();

        let mut files: Vec<File> = data
            .files
            .iter()
            .filter(|f| f.account == user_account && bucket_ids.contains(&f.bucket_id))
            .cloned()
            .collect();

        // Apply pagination
        if let Some(params) = pagination {
            let offset = params.offset.unwrap_or(0) as usize;
            let limit = params.limit.unwrap_or(20) as usize;
            files = files.into_iter().skip(offset).take(limit).collect();
        }

        Ok(files)
    }

    async fn get_files_by_bucket_id(
        &self,
        bucket_id: i64,
        pagination: Option<PaginationParams>,
    ) -> Result<Vec<File>> {
        let data = self.data.lock().unwrap();
        let mut files: Vec<File> = data
            .files
            .iter()
            .filter(|f| f.bucket_id == bucket_id)
            .cloned()
            .collect();

        // Apply pagination
        if let Some(params) = pagination {
            let offset = params.offset.unwrap_or(0) as usize;
            let limit = params.limit.unwrap_or(20) as usize;
            files = files.into_iter().skip(offset).take(limit).collect();
        }

        Ok(files)
    }

    async fn create_file(&self, file: File) -> Result<File> {
        let mut data = self.data.lock().unwrap();
        
        // Check if file already exists
        if data.files.iter().any(|f| f.file_key == file.file_key) {
            return Err(Error::Database("File already exists".to_string()));
        }

        // Generate a new ID
        let new_id = data.files.iter().map(|f| f.id).max().unwrap_or(0) + 1;
        let mut new_file = file;
        new_file.id = new_id;
        
        data.files.push(new_file.clone());
        Ok(new_file)
    }

    async fn update_file_step(
        &self,
        file_key: &[u8],
        step: FileStorageRequestStep,
    ) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        match data.files.iter_mut().find(|f| f.file_key == file_key) {
            Some(file) => {
                file.step = step as i32;
                file.updated_at = NaiveDateTime::from_timestamp_opt(
                    chrono::Utc::now().timestamp(),
                    0,
                )
                .unwrap();
                Ok(())
            }
            None => Err(Error::NotFound("File not found".to_string())),
        }
    }

    async fn delete_file(&self, file_key: &[u8]) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        let original_len = data.files.len();
        data.files.retain(|f| f.file_key != file_key);
        
        if data.files.len() < original_len {
            Ok(())
        } else {
            Err(Error::NotFound("File not found".to_string()))
        }
    }

    async fn get_bucket_by_id(&self, bucket_id: i64) -> Result<Bucket> {
        let data = self.data.lock().unwrap();
        data.buckets
            .iter()
            .find(|b| b.id == bucket_id)
            .cloned()
            .ok_or_else(|| Error::NotFound("Bucket not found".to_string()))
    }

    async fn get_buckets_by_user(
        &self,
        user_account: &[u8],
        pagination: Option<PaginationParams>,
    ) -> Result<Vec<Bucket>> {
        let data = self.data.lock().unwrap();
        let account = hex::encode(user_account);
        let mut buckets: Vec<Bucket> = data
            .buckets
            .iter()
            .filter(|b| b.account == account)
            .cloned()
            .collect();

        // Apply pagination
        if let Some(params) = pagination {
            let offset = params.offset.unwrap_or(0) as usize;
            let limit = params.limit.unwrap_or(20) as usize;
            buckets = buckets.into_iter().skip(offset).take(limit).collect();
        }

        Ok(buckets)
    }

    async fn get_msp_by_id(&self, msp_id: i64) -> Result<Msp> {
        let data = self.data.lock().unwrap();
        data.msps
            .iter()
            .find(|m| m.id == msp_id)
            .cloned()
            .ok_or_else(|| Error::NotFound("MSP not found".to_string()))
    }

    async fn get_all_msps(&self, pagination: Option<PaginationParams>) -> Result<Vec<Msp>> {
        let data = self.data.lock().unwrap();
        let mut msps = data.msps.clone();

        // Apply pagination
        if let Some(params) = pagination {
            let offset = params.offset.unwrap_or(0) as usize;
            let limit = params.limit.unwrap_or(20) as usize;
            msps = msps.into_iter().skip(offset).take(limit).collect();
        }

        Ok(msps)
    }

    async fn execute_raw_query(&self, _query: &str) -> Result<Vec<serde_json::Value>> {
        // For mock, return empty results
        Ok(vec![])
    }
}