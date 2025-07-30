# Implementation Plan: StorageHub MSP Mock API in Existing Backend

## ⚠️ IMPORTANT COMPILATION NOTE

When compiling or checking this project, you **MUST** run commands from the workspace root WITHOUT specifying any specific package:

✅ **CORRECT**: `cargo check`  
✅ **CORRECT**: `cargo build`  
✅ **CORRECT**: `cargo run --bin backend`  

❌ **INCORRECT**: `cargo check -p sh-backend-lib`  
❌ **INCORRECT**: `cargo build -p backend`  

Always run cargo commands from the workspace root to ensure proper dependency resolution.

---

## Overview

Extend the existing Axum backend in mock-endpoints/backend to implement all StorageHub MSP endpoints with mock responses, leveraging the current architecture and mock infrastructure.

## Prerequisites

- [x] Existing backend project with Axum framework
- [x] Mock infrastructure for testing (MockDbConnection, MockConnection)
- [x] Modular architecture with api/, services/, and data/ layers
- [ ] Update dependencies for JWT and multipart support

## Steps

### 1. Update Dependencies

- File: `mock-endpoints/backend/lib/Cargo.toml`
- Operation: Add required dependencies after line 31 (in dependencies section)
- Details:
  ```toml
  jsonwebtoken = "9"
  axum-extra = { version = "0.9", features = ["multipart"] }
  hex = "0.4"
  rand = "0.8"
  chrono = { version = "0.4", features = ["serde"] }
  base64 = "0.21"
  ```
- Success: cargo check passes with new dependencies

### 2. Create MSP Models Module

- File: `mock-endpoints/backend/lib/src/models/mod.rs` (create new)
- Operation: Define all MSP API request/response models
- Details:
  ```rust
  pub mod auth;
  pub mod msp_info;
  pub mod buckets;
  pub mod files;
  pub mod payment;
  
  pub use auth::*;
  pub use msp_info::*;
  pub use buckets::*;
  pub use files::*;
  pub use payment::*;
  ```
- Success: Module structure created

### 3. Define Authentication Models

- File: `mock-endpoints/backend/lib/src/models/auth.rs` (create new)
- Operation: Create auth-related request/response types
- Details:
  ```rust
  use serde::{Deserialize, Serialize};

  #[derive(Debug, Deserialize)]
  pub struct NonceRequest {
      pub address: String,
      pub chain_id: u64,
  }

  #[derive(Debug, Serialize)]
  pub struct NonceResponse {
      pub message: String,
      pub nonce: String,
  }

  #[derive(Debug, Deserialize)]
  pub struct VerifyRequest {
      pub message: String,
      pub signature: String,
  }

  #[derive(Debug, Serialize)]
  pub struct VerifyResponse {
      pub token: String,
      pub user: User,
  }

  #[derive(Debug, Serialize)]
  pub struct User {
      pub address: String,
  }

  #[derive(Debug, Serialize)]
  pub struct ProfileResponse {
      pub address: String,
      pub ens: String,
  }

  #[derive(Debug, Serialize)]
  pub struct TokenResponse {
      pub token: String,
  }
  ```
- Success: Auth models compile

### 4. Define MSP Info Models

- File: `mock-endpoints/backend/lib/src/models/msp_info.rs` (create new)
- Operation: Create MSP information types
- Details:
  ```rust
  use serde::Serialize;
  use chrono::{DateTime, Utc};

  #[derive(Debug, Serialize)]
  pub struct InfoResponse {
      pub client: String,
      pub version: String,
      #[serde(rename = "mspId")]
      pub msp_id: String,
      pub multiaddresses: Vec<String>,
      #[serde(rename = "ownerAccount")]
      pub owner_account: String,
      #[serde(rename = "paymentAccount")]
      pub payment_account: String,
      pub status: String,
      #[serde(rename = "activeSince")]
      pub active_since: u64,
      pub uptime: String,
  }

  #[derive(Debug, Serialize)]
  pub struct StatsResponse {
      pub capacity: Capacity,
      #[serde(rename = "activeUsers")]
      pub active_users: u64,
      #[serde(rename = "lastCapacityChange")]
      pub last_capacity_change: u64,
      #[serde(rename = "valuePropsAmount")]
      pub value_props_amount: u64,
      #[serde(rename = "BucketsAmount")]
      pub buckets_amount: u64,
  }

  #[derive(Debug, Serialize)]
  pub struct Capacity {
      #[serde(rename = "totalBytes")]
      pub total_bytes: u64,
      #[serde(rename = "availableBytes")]
      pub available_bytes: u64,
      #[serde(rename = "usedBytes")]
      pub used_bytes: u64,
  }

  #[derive(Debug, Serialize)]
  pub struct ValueProp {
      pub id: String,
      #[serde(rename = "pricePerGbBlock")]
      pub price_per_gb_block: f64,
      #[serde(rename = "dataLimitPerBucketBytes")]
      pub data_limit_per_bucket_bytes: u64,
      #[serde(rename = "isAvailable")]
      pub is_available: bool,
  }

  #[derive(Debug, Serialize)]
  pub struct MspHealthResponse {
      pub status: String,
      pub components: serde_json::Value,
      #[serde(rename = "lastChecked")]
      pub last_checked: DateTime<Utc>,
  }
  ```
- Success: MSP info models defined

### 5. Define Bucket and File Models

- File: `mock-endpoints/backend/lib/src/models/buckets.rs` (create new)
- Operation: Create bucket-related types
- Details:
  ```rust
  use serde::Serialize;

  #[derive(Debug, Serialize)]
  pub struct Bucket {
      #[serde(rename = "bucketId")]
      pub bucket_id: String,
      pub name: String,
      pub root: String,
      #[serde(rename = "isPublic")]
      pub is_public: bool,
      #[serde(rename = "sizeBytes")]
      pub size_bytes: u64,
      #[serde(rename = "valuePropId")]
      pub value_prop_id: String,
      #[serde(rename = "fileCount")]
      pub file_count: u64,
  }

  #[derive(Debug, Serialize)]
  pub struct FileTree {
      pub name: String,
      #[serde(rename = "type")]
      pub node_type: String,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub children: Option<Vec<FileTree>>,
      #[serde(skip_serializing_if = "Option::is_none", rename = "sizeBytes")]
      pub size_bytes: Option<u64>,
      #[serde(skip_serializing_if = "Option::is_none", rename = "fileKey")]
      pub file_key: Option<String>,
  }
  ```
- Success: Bucket models created

### 6. Define File Operation Models

- File: `mock-endpoints/backend/lib/src/models/files.rs` (create new)
- Operation: Create file-related types
- Details:
  ```rust
  use serde::Serialize;
  use chrono::{DateTime, Utc};

  #[derive(Debug, Serialize)]
  pub struct FileInfo {
      #[serde(rename = "fileKey")]
      pub file_key: String,
      pub fingerprint: String,
      #[serde(rename = "bucketId")]
      pub bucket_id: String,
      pub name: String,
      pub location: String,
      pub size: u64,
      #[serde(rename = "isPublic")]
      pub is_public: bool,
      #[serde(rename = "uploadedAt")]
      pub uploaded_at: DateTime<Utc>,
  }

  #[derive(Debug, Serialize)]
  pub struct DistributeResponse {
      pub status: String,
      #[serde(rename = "fileKey")]
      pub file_key: String,
      pub message: String,
  }
  ```
- Success: File models defined

### 7. Define Payment Models

- File: `mock-endpoints/backend/lib/src/models/payment.rs` (create new)
- Operation: Create payment stream type
- Details:
  ```rust
  use serde::Serialize;

  #[derive(Debug, Serialize)]
  pub struct PaymentStream {
      #[serde(rename = "tokensPerBlock")]
      pub tokens_per_block: u64,
      #[serde(rename = "lastChargedTick")]
      pub last_charged_tick: u64,
      #[serde(rename = "userDeposit")]
      pub user_deposit: u64,
      #[serde(rename = "outOfFundsTick")]
      pub out_of_funds_tick: Option<u64>,
  }
  ```
- Success: Payment model created

### 8. Update lib.rs to Include Models

- File: `mock-endpoints/backend/lib/src/lib.rs`
- Operation: Add models module export after line 4
- Details:
  ```rust
  pub mod models;
  ```
- Success: Models module accessible

### 9. Create MSP Validation Utilities

- File: `mock-endpoints/backend/lib/src/api/validation.rs` (create new)
- Operation: Add input validation helpers
- Details:
  ```rust
  use crate::error::Error;

  pub fn validate_eth_address(address: &str) -> Result<(), Error> {
      if address.starts_with("0x") && address.len() == 42 && 
         address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
          Ok(())
      } else {
          Err(Error::BadRequest("Invalid Ethereum address".to_string()))
      }
  }

  pub fn validate_hex_id(id: &str, expected_len: usize) -> Result<(), Error> {
      if id.len() == expected_len && id.chars().all(|c| c.is_ascii_hexdigit()) {
          Ok(())
      } else {
          Err(Error::BadRequest(format!("Invalid hex ID, expected {} characters", expected_len)))
      }
  }

  pub fn generate_hex_string(len: usize) -> String {
      use rand::Rng;
      let mut rng = rand::thread_rng();
      (0..len/2)
          .map(|_| format!("{:02x}", rng.gen::<u8>()))
          .collect()
  }

  pub fn generate_mock_jwt() -> String {
      use base64::{Engine, engine::general_purpose};
      format!("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.{}.{}",
          general_purpose::STANDARD.encode("mock_payload"),
          general_purpose::STANDARD.encode("mock_signature")
      )
  }

  pub fn extract_bearer_token(auth_header: Option<&str>) -> Result<String, Error> {
      match auth_header {
          Some(header) if header.starts_with("Bearer ") => {
              Ok(header[7..].to_string())
          }
          _ => Err(Error::Unauthorized("Missing or invalid authorization header".to_string()))
      }
  }
  ```
- Success: Validation utilities compile

### 10. Create MSP Service

- File: `mock-endpoints/backend/lib/src/services/msp.rs` (create new)
- Operation: Create service layer for MSP operations
- Details:
  ```rust
  use crate::models::*;
  use crate::error::Error;
  use chrono::Utc;

  #[derive(Clone)]
  pub struct MspService {
      // In a real implementation, this would have database connections
  }

  impl MspService {
      pub fn new() -> Self {
          Self {}
      }

      pub async fn get_info(&self) -> Result<InfoResponse, Error> {
          Ok(InfoResponse {
              client: "storagehub-node v1.0.0".to_string(),
              version: "StorageHub MSP v0.1.0".to_string(),
              msp_id: "4c310f61f81475048e8ce5eadf4ee718c42ba285579bb37ac6da55a92c638f42".to_string(),
              multiaddresses: vec![
                  "/ip4/192.168.0.10/tcp/30333/p2p/12D3KooWJAgnKUrQkGsKxRxojxcFRhtH6ovWfJTPJjAkhmAz2yC8".to_string()
              ],
              owner_account: "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac".to_string(),
              payment_account: "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac".to_string(),
              status: "active".to_string(),
              active_since: 123,
              uptime: "2 days, 1 hour".to_string(),
          })
      }

      pub async fn get_stats(&self) -> Result<StatsResponse, Error> {
          Ok(StatsResponse {
              capacity: Capacity {
                  total_bytes: 1099511627776,
                  available_bytes: 879609302220,
                  used_bytes: 219902325556,
              },
              active_users: 152,
              last_capacity_change: 123,
              value_props_amount: 42,
              buckets_amount: 1024,
          })
      }

      pub async fn get_value_props(&self) -> Result<Vec<ValueProp>, Error> {
          Ok(vec![
              ValueProp {
                  id: "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32".to_string(),
                  price_per_gb_block: 0.5,
                  data_limit_per_bucket_bytes: 10737418240,
                  is_available: true,
              }
          ])
      }

      pub async fn get_health(&self) -> Result<MspHealthResponse, Error> {
          Ok(MspHealthResponse {
              status: "healthy".to_string(),
              components: serde_json::json!({
                  "database": {
                      "status": "healthy",
                      "details": "PostgreSQL connection active"
                  },
                  "mspClient": {
                      "status": "healthy",
                      "details": "Connected to StorageHub MSP client"
                  },
                  "storageHubNetwork": {
                      "status": "healthy",
                      "details": "Node synced with network"
                  }
              }),
              last_checked: Utc::now(),
          })
      }

      pub async fn list_user_buckets(&self, _user_address: &str) -> Result<Vec<Bucket>, Error> {
          Ok(vec![
              Bucket {
                  bucket_id: "d8793e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26692".to_string(),
                  name: "Documents".to_string(),
                  root: "3de0c6d1959ece558ec030f37292e383a9c95f497e8235b89701b914be9bd1fb".to_string(),
                  is_public: false,
                  size_bytes: 12345678,
                  value_prop_id: "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32".to_string(),
                  file_count: 12,
              }
          ])
      }

      pub async fn get_bucket(&self, bucket_id: &str) -> Result<Bucket, Error> {
          Ok(Bucket {
              bucket_id: bucket_id.to_string(),
              name: "Documents".to_string(),
              root: "3de0c6d1959ece558ec030f37292e383a9c95f497e8235b89701b914be9bd1fb".to_string(),
              is_public: false,
              size_bytes: 12345678,
              value_prop_id: "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32".to_string(),
              file_count: 12,
          })
      }

      pub async fn get_file_tree(&self, _bucket_id: &str) -> Result<FileTree, Error> {
          Ok(FileTree {
              name: "/".to_string(),
              node_type: "folder".to_string(),
              children: Some(vec![
                  FileTree {
                      name: "Thesis".to_string(),
                      node_type: "folder".to_string(),
                      children: Some(vec![
                          FileTree {
                              name: "Initial_results.png".to_string(),
                              node_type: "file".to_string(),
                              children: None,
                              size_bytes: Some(54321),
                              file_key: Some("d298c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f2".to_string()),
                          }
                      ]),
                      size_bytes: None,
                      file_key: None,
                  }
              ]),
              size_bytes: None,
              file_key: None,
          })
      }

      pub async fn get_file_info(&self, bucket_id: &str, file_key: &str) -> Result<FileInfo, Error> {
          Ok(FileInfo {
              file_key: file_key.to_string(),
              fingerprint: "5d7a3700e1f7d973c064539f1b18c988dace6b4f1a57650165e9b58305db090f".to_string(),
              bucket_id: bucket_id.to_string(),
              name: "Q1-2024.pdf".to_string(),
              location: "/files/documents/reports".to_string(),
              size: 54321,
              is_public: true,
              uploaded_at: Utc::now(),
          })
      }

      pub async fn distribute_file(&self, _bucket_id: &str, file_key: &str) -> Result<DistributeResponse, Error> {
          Ok(DistributeResponse {
              status: "distribution_initiated".to_string(),
              file_key: file_key.to_string(),
              message: "File distribution to volunteering BSPs has been initiated".to_string(),
          })
      }

      pub async fn get_payment_stream(&self, _user_address: &str) -> Result<PaymentStream, Error> {
          Ok(PaymentStream {
              tokens_per_block: 100,
              last_charged_tick: 1234567,
              user_deposit: 100000,
              out_of_funds_tick: None,
          })
      }
  }
  ```
- Success: MSP service with mock implementations

### 11. Create Auth Service

- File: `mock-endpoints/backend/lib/src/services/auth.rs` (create new)
- Operation: Create authentication service
- Details:
  ```rust
  use crate::models::*;
  use crate::error::Error;
  use crate::api::validation::{validate_eth_address, generate_mock_jwt};

  #[derive(Clone)]
  pub struct AuthService {
      // In real implementation, would store nonces and sessions
  }

  impl AuthService {
      pub fn new() -> Self {
          Self {}
      }

      pub async fn generate_nonce(&self, address: &str, chain_id: u64) -> Result<NonceResponse, Error> {
          validate_eth_address(address)?;
          
          Ok(NonceResponse {
              message: format!(
                  "example.com wants you to sign in with your Ethereum account:\n{}\n\n\
                  Sign in to access your account.\n\n\
                  URI: https://example.com\n\
                  Version: 1\n\
                  Chain ID: {}\n\
                  Nonce: aBcDeF12345\n\
                  Issued At: 2025-07-01T11:58:00.000Z",
                  address, chain_id
              ),
              nonce: "aBcDeF12345".to_string(),
          })
      }

      pub async fn verify_signature(&self, _message: &str, signature: &str) -> Result<VerifyResponse, Error> {
          if !signature.starts_with("0x") || signature.len() != 132 {
              return Err(Error::Unauthorized("Invalid signature".to_string()));
          }

          Ok(VerifyResponse {
              token: generate_mock_jwt(),
              user: User {
                  address: "0x1234567890123456789012345678901234567890".to_string(),
              },
          })
      }

      pub async fn refresh_token(&self, _old_token: &str) -> Result<TokenResponse, Error> {
          Ok(TokenResponse {
              token: generate_mock_jwt(),
          })
      }

      pub async fn get_profile(&self, _token: &str) -> Result<ProfileResponse, Error> {
          Ok(ProfileResponse {
              address: "0x1234567890123456789012345678901234567890".to_string(),
              ens: "user.eth".to_string(),
          })
      }

      pub async fn logout(&self, _token: &str) -> Result<(), Error> {
          // In real implementation, would invalidate the token
          Ok(())
      }
  }
  ```
- Success: Auth service created

### 12. Update Services Module

- File: `mock-endpoints/backend/lib/src/services/mod.rs`
- Operation: Add new services after line 3
- Details:
  ```rust
  pub mod auth;
  pub mod msp;

  use auth::AuthService;
  use msp::MspService;
  ```
  And update the Services struct (around line 15):
  ```rust
  pub struct Services {
      pub health: HealthService,
      pub counter: CounterService,
      pub auth: AuthService,
      pub msp: MspService,
  }

  impl Services {
      pub fn new() -> Self {
          Self {
              health: HealthService::new(),
              counter: CounterService::new(),
              auth: AuthService::new(),
              msp: MspService::new(),
          }
      }
  }
  ```
- Success: Services updated with new modules

### 13. Create MSP Handlers

- File: `mock-endpoints/backend/lib/src/api/msp_handlers.rs` (create new)
- Operation: Create HTTP handlers for MSP endpoints
- Details:
  ```rust
  use axum::{
      extract::{Path, State},
      http::{StatusCode, HeaderMap},
      response::IntoResponse,
      Json,
  };
  use axum_extra::extract::Multipart;
  use crate::{
      services::Services,
      models::*,
      api::validation::*,
      error::Error,
  };

  // Auth handlers
  pub async fn nonce(
      State(services): State<Services>,
      Json(payload): Json<NonceRequest>,
  ) -> Result<Json<NonceResponse>, Error> {
      let response = services.auth.generate_nonce(&payload.address, payload.chain_id).await?;
      Ok(Json(response))
  }

  pub async fn verify(
      State(services): State<Services>,
      Json(payload): Json<VerifyRequest>,
  ) -> Result<Json<VerifyResponse>, Error> {
      let response = services.auth.verify_signature(&payload.message, &payload.signature).await?;
      Ok(Json(response))
  }

  pub async fn refresh(
      State(services): State<Services>,
      headers: HeaderMap,
  ) -> Result<Json<TokenResponse>, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let token = extract_bearer_token(auth_header)?;
      
      let response = services.auth.refresh_token(&token).await?;
      Ok(Json(response))
  }

  pub async fn logout(
      State(services): State<Services>,
      headers: HeaderMap,
  ) -> Result<impl IntoResponse, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let token = extract_bearer_token(auth_header)?;
      
      services.auth.logout(&token).await?;
      Ok(StatusCode::NO_CONTENT)
  }

  pub async fn profile(
      State(services): State<Services>,
      headers: HeaderMap,
  ) -> Result<Json<ProfileResponse>, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let token = extract_bearer_token(auth_header)?;
      
      let response = services.auth.get_profile(&token).await?;
      Ok(Json(response))
  }

  // MSP info handlers
  pub async fn info(State(services): State<Services>) -> Result<Json<InfoResponse>, Error> {
      let response = services.msp.get_info().await?;
      Ok(Json(response))
  }

  pub async fn stats(State(services): State<Services>) -> Result<Json<StatsResponse>, Error> {
      let response = services.msp.get_stats().await?;
      Ok(Json(response))
  }

  pub async fn value_props(State(services): State<Services>) -> Result<Json<Vec<ValueProp>>, Error> {
      let response = services.msp.get_value_props().await?;
      Ok(Json(response))
  }

  pub async fn msp_health(State(services): State<Services>) -> Result<Json<MspHealthResponse>, Error> {
      let response = services.msp.get_health().await?;
      Ok(Json(response))
  }

  // Bucket handlers
  pub async fn list_buckets(
      State(services): State<Services>,
      headers: HeaderMap,
  ) -> Result<Json<Vec<Bucket>>, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      // In real implementation, would extract user from token
      let response = services.msp.list_user_buckets("mock_user").await?;
      Ok(Json(response))
  }

  pub async fn get_bucket(
      State(services): State<Services>,
      Path(bucket_id): Path<String>,
      headers: HeaderMap,
  ) -> Result<Json<Bucket>, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      validate_hex_id(&bucket_id, 64)?;
      let response = services.msp.get_bucket(&bucket_id).await?;
      Ok(Json(response))
  }

  pub async fn get_files(
      State(services): State<Services>,
      Path(bucket_id): Path<String>,
      headers: HeaderMap,
  ) -> Result<Json<FileTree>, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      validate_hex_id(&bucket_id, 64)?;
      let response = services.msp.get_file_tree(&bucket_id).await?;
      Ok(Json(response))
  }

  // File handlers
  pub async fn download_by_location(
      Path((bucket_id, file_location)): Path<(String, String)>,
      headers: HeaderMap,
  ) -> Result<impl IntoResponse, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      validate_hex_id(&bucket_id, 64)?;
      
      // Return mock file data
      Ok((
          StatusCode::OK,
          [("content-type", "application/octet-stream")],
          b"Mock file content for testing".to_vec(),
      ))
  }

  pub async fn download_by_key(
      Path((bucket_id, file_key)): Path<(String, String)>,
      headers: HeaderMap,
  ) -> Result<impl IntoResponse, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      validate_hex_id(&bucket_id, 64)?;
      validate_hex_id(&file_key, 64)?;
      
      Ok((
          StatusCode::OK,
          [("content-type", "application/octet-stream")],
          b"Mock file content for testing".to_vec(),
      ))
  }

  pub async fn get_file_info(
      State(services): State<Services>,
      Path((bucket_id, file_key)): Path<(String, String)>,
      headers: HeaderMap,
  ) -> Result<Json<FileInfo>, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      validate_hex_id(&bucket_id, 64)?;
      validate_hex_id(&file_key, 64)?;
      
      let response = services.msp.get_file_info(&bucket_id, &file_key).await?;
      Ok(Json(response))
  }

  pub async fn upload_file(
      State(services): State<Services>,
      Path((bucket_id, file_key)): Path<(String, String)>,
      headers: HeaderMap,
      mut multipart: Multipart,
  ) -> Result<impl IntoResponse, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      validate_hex_id(&bucket_id, 64)?;
      validate_hex_id(&file_key, 64)?;
      
      // Mock file processing
      while let Some(field) = multipart.next_field().await.unwrap() {
          let _data = field.bytes().await.unwrap();
          // In real implementation, would process the file
      }
      
      let response = services.msp.get_file_info(&bucket_id, &file_key).await?;
      Ok((StatusCode::CREATED, Json(response)))
  }

  pub async fn distribute_file(
      State(services): State<Services>,
      Path((bucket_id, file_key)): Path<(String, String)>,
      headers: HeaderMap,
  ) -> Result<Json<DistributeResponse>, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      validate_hex_id(&bucket_id, 64)?;
      validate_hex_id(&file_key, 64)?;
      
      let response = services.msp.distribute_file(&bucket_id, &file_key).await?;
      Ok(Json(response))
  }

  // Payment handler
  pub async fn payment_stream(
      State(services): State<Services>,
      headers: HeaderMap,
  ) -> Result<Json<PaymentStream>, Error> {
      let auth_header = headers.get("authorization")
          .and_then(|h| h.to_str().ok());
      let _token = extract_bearer_token(auth_header)?;
      
      // In real implementation, would extract user from token
      let response = services.msp.get_payment_stream("mock_user").await?;
      Ok(Json(response))
  }
  ```
- Success: All MSP handlers implemented

### 14. Update API Module

- File: `mock-endpoints/backend/lib/src/api/mod.rs`
- Operation: Add validation and msp_handlers modules after line 3
- Details:
  ```rust
  pub mod validation;
  pub mod msp_handlers;
  ```
- Success: New modules exported

### 15. Update Routes

- File: `mock-endpoints/backend/lib/src/api/routes.rs`
- Operation: Add MSP routes after line 10 (after existing routes)
- Details:
  ```rust
  use crate::api::msp_handlers;
  
  // Then in the routes() function, after the existing routes:
  
  // Auth routes
  .route("/auth/nonce", post(msp_handlers::nonce))
  .route("/auth/verify", post(msp_handlers::verify))
  .route("/auth/refresh", post(msp_handlers::refresh))
  .route("/auth/logout", post(msp_handlers::logout))
  .route("/auth/profile", get(msp_handlers::profile))
  
  // MSP info routes
  .route("/info", get(msp_handlers::info))
  .route("/stats", get(msp_handlers::stats))
  .route("/value-props", get(msp_handlers::value_props))
  .route("/health", get(msp_handlers::msp_health))
  
  // Bucket routes
  .route("/buckets", get(msp_handlers::list_buckets))
  .route("/buckets/:bucket_id", get(msp_handlers::get_bucket))
  .route("/buckets/:bucket_id/files", get(msp_handlers::get_files))
  
  // File routes - note the order matters for path matching
  .route("/buckets/:bucket_id/:file_key/info", get(msp_handlers::get_file_info))
  .route("/buckets/:bucket_id/:file_key/upload", put(msp_handlers::upload_file))
  .route("/buckets/:bucket_id/:file_key/distribute", post(msp_handlers::distribute_file))
  .route("/buckets/:bucket_id/:file_key", get(msp_handlers::download_by_key))
  .route("/buckets/:bucket_id/*file_location", get(msp_handlers::download_by_location))
  
  // Payment route
  .route("/payment_stream", get(msp_handlers::payment_stream))
  ```
- Success: All MSP routes registered

### 16. Update Error Handling

- File: `mock-endpoints/backend/lib/src/error.rs`
- Operation: Add new error variants after line 20 (in Error enum)
- Details:
  ```rust
  #[error("Bad request: {0}")]
  BadRequest(String),
  
  #[error("Unauthorized: {0}")]
  Unauthorized(String),
  
  #[error("Forbidden: {0}")]
  Forbidden(String),
  
  #[error("Not found: {0}")]
  NotFound(String),
  
  #[error("Conflict: {0}")]
  Conflict(String),
  ```
  And update the IntoResponse implementation to handle new variants (around line 50):
  ```rust
  Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
  Error::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
  Error::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
  Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
  Error::Conflict(msg) => (StatusCode::CONFLICT, msg),
  ```
- Success: Error types support all required status codes

## Testing Strategy

- [ ] Run `cargo check` from workspace root to verify compilation
- [ ] Start server with `cargo run --bin backend` from workspace root
- [ ] Test public endpoints: `curl http://localhost:8080/info`
- [ ] Test auth flow: POST /auth/nonce → POST /auth/verify
- [ ] Test authenticated endpoint: `curl -H "Authorization: Bearer test" http://localhost:8080/buckets`
- [ ] Test file upload: `curl -X PUT -H "Authorization: Bearer test" -F "file=@test.txt" http://localhost:8080/buckets/{bucket_id}/{file_key}/upload`
- [ ] Verify CORS headers in responses

## Rollback Plan

1. Remove all new files created in this plan
2. Revert changes to existing files (lib.rs, services/mod.rs, api/mod.rs, api/routes.rs, error.rs, Cargo.toml)
3. Run `cargo clean` to remove build artifacts