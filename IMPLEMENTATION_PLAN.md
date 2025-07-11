# Implementation Plan: Add Remote File Protocol Support with Trait-Based Architecture

## Overview

Extend the `loadFileInStorage` and `saveFileToDisk` RPC methods to support remote file protocols through a trait-based architecture that enables easy addition of new protocol handlers while maintaining backward compatibility.

## Prerequisites

- [ ] Add `reqwest = { version = "0.11", features = ["stream", "rustls-tls"] }` to `/client/rpc/Cargo.toml`
- [ ] Add `url = "2.5"` to `/client/rpc/Cargo.toml`
- [ ] Add `suppaftp = { version = "6.0", features = ["async-tokio", "rustls"] }` to `/client/rpc/Cargo.toml`
- [ ] Add `tokio-util = { version = "0.7", features = ["io"] }` to `/client/rpc/Cargo.toml`
- [ ] Add `bytes = "1.5"` to `/client/rpc/Cargo.toml`
- [ ] Existing RPC implementation at `/client/rpc/src/lib.rs`

## Architecture Design

The trait-based architecture will consist of:
- `RemoteFileHandler` trait defining the interface
- Protocol-specific implementations (Local, HTTP/HTTPS, FTP)
- Handler registry for scheme-based handler selection
- Async streaming support for efficient large file handling

## Steps

### 1. Create RemoteFileHandler Trait

- File: `/client/rpc/src/remote_file/mod.rs` (create new)
- Operation: Define the core trait and module structure
- Details:
  ```rust
  use async_trait::async_trait;
  use futures::stream::Stream;
  use std::pin::Pin;
  use anyhow::Result;
  
  pub mod local;
  pub mod http;
  pub mod ftp;
  pub mod registry;
  
  pub use registry::HandlerRegistry;
  
  /// Trait for handling remote file operations
  #[async_trait]
  pub trait RemoteFileHandler: Send + Sync {
      /// Check if this handler supports the given URI scheme
      fn supports_scheme(&self, scheme: &str) -> bool;
      
      /// Download a file from the given URI and return a stream of chunks
      async fn download(
          &self,
          uri: &str,
      ) -> Result<(u64, Pin<Box<dyn Stream<Item = Result<bytes::Bytes>> + Send>>)>;
      
      /// Upload data to the given URI (returns error if not supported)
      async fn upload(
          &self,
          uri: &str,
          data: Pin<Box<dyn Stream<Item = Result<bytes::Bytes>> + Send>>,
      ) -> Result<()>;
      
      /// Check if upload is supported for this handler
      fn supports_upload(&self) -> bool {
          false
      }
  }
  ```
- Success: Trait compiles with async-trait support

### 2. Implement LocalFileHandler

- File: `/client/rpc/src/remote_file/local.rs` (create new)
- Operation: Implement local file system handler
- Details:
  ```rust
  use super::RemoteFileHandler;
  use async_trait::async_trait;
  use futures::stream::{self, Stream, StreamExt};
  use std::pin::Pin;
  use std::path::PathBuf;
  use tokio::fs::File;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use bytes::Bytes;
  use anyhow::{Result, Context};
  
  pub struct LocalFileHandler;
  
  #[async_trait]
  impl RemoteFileHandler for LocalFileHandler {
      fn supports_scheme(&self, scheme: &str) -> bool {
          scheme.is_empty() || scheme == "file"
      }
      
      async fn download(
          &self,
          uri: &str,
      ) -> Result<(u64, Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>)> {
          // Remove file:// prefix if present
          let path = if uri.starts_with("file://") {
              &uri[7..]
          } else {
              uri
          };
          
          let file_path = PathBuf::from(path);
          let mut file = File::open(&file_path)
              .await
              .context("Failed to open file")?;
          
          let metadata = file.metadata()
              .await
              .context("Failed to get file metadata")?;
          let file_size = metadata.len();
          
          // Read file in chunks
          let chunk_size = 1024 * 1024; // 1MB chunks
          let chunks = stream::unfold(file, move |mut file| async move {
              let mut buffer = vec![0u8; chunk_size];
              match file.read(&mut buffer).await {
                  Ok(0) => None,
                  Ok(n) => {
                      buffer.truncate(n);
                      Some((Ok(Bytes::from(buffer)), file))
                  }
                  Err(e) => Some((Err(anyhow::anyhow!("Read error: {}", e)), file))
              }
          });
          
          Ok((file_size, Box::pin(chunks)))
      }
      
      async fn upload(
          &self,
          uri: &str,
          mut data: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
      ) -> Result<()> {
          let path = if uri.starts_with("file://") {
              &uri[7..]
          } else {
              uri
          };
          
          let file_path = PathBuf::from(path);
          
          // Create parent directories if needed
          if let Some(parent) = file_path.parent() {
              tokio::fs::create_dir_all(parent)
                  .await
                  .context("Failed to create parent directories")?;
          }
          
          let mut file = File::create(&file_path)
              .await
              .context("Failed to create file")?;
          
          while let Some(chunk) = data.next().await {
              let chunk = chunk?;
              file.write_all(&chunk)
                  .await
                  .context("Failed to write chunk")?;
          }
          
          file.flush().await.context("Failed to flush file")?;
          Ok(())
      }
      
      fn supports_upload(&self) -> bool {
          true
      }
  }
  ```
- Success: Local file operations work with streaming

### 3. Implement HttpFileHandler

- File: `/client/rpc/src/remote_file/http.rs` (create new)
- Operation: Implement HTTP/HTTPS handler
- Details:
  ```rust
  use super::RemoteFileHandler;
  use async_trait::async_trait;
  use futures::stream::{Stream, StreamExt};
  use std::pin::Pin;
  use bytes::Bytes;
  use anyhow::{Result, Context};
  use reqwest::Client;
  
  pub struct HttpFileHandler {
      client: Client,
  }
  
  impl HttpFileHandler {
      pub fn new() -> Self {
          Self {
              client: Client::builder()
                  .timeout(std::time::Duration::from_secs(300))
                  .build()
                  .expect("Failed to create HTTP client"),
          }
      }
  }
  
  #[async_trait]
  impl RemoteFileHandler for HttpFileHandler {
      fn supports_scheme(&self, scheme: &str) -> bool {
          scheme == "http" || scheme == "https"
      }
      
      async fn download(
          &self,
          uri: &str,
      ) -> Result<(u64, Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>)> {
          let response = self.client
              .get(uri)
              .send()
              .await
              .context("Failed to send HTTP request")?;
          
          if !response.status().is_success() {
              return Err(anyhow::anyhow!(
                  "HTTP error {}: {}",
                  response.status().as_u16(),
                  response.status().canonical_reason().unwrap_or("Unknown")
              ));
          }
          
          let content_length = response
              .content_length()
              .ok_or_else(|| anyhow::anyhow!("No content-length header"))?;
          
          let stream = response
              .bytes_stream()
              .map(|result| result
                  .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
              );
          
          Ok((content_length, Box::pin(stream)))
      }
      
      async fn upload(
          &self,
          _uri: &str,
          _data: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
      ) -> Result<()> {
          Err(anyhow::anyhow!("HTTP upload not supported"))
      }
  }
  ```
- Success: HTTP/HTTPS downloads work with proper error handling

### 4. Implement FtpFileHandler

- File: `/client/rpc/src/remote_file/ftp.rs` (create new)
- Operation: Implement FTP handler with URI-based auth
- Details:
  ```rust
  use super::RemoteFileHandler;
  use async_trait::async_trait;
  use futures::stream::{self, Stream};
  use std::pin::Pin;
  use bytes::Bytes;
  use anyhow::{Result, Context};
  use suppaftp::AsyncFtpStream;
  use tokio_util::compat::TokioAsyncReadCompatExt;
  use url::Url;
  
  pub struct FtpFileHandler;
  
  #[async_trait]
  impl RemoteFileHandler for FtpFileHandler {
      fn supports_scheme(&self, scheme: &str) -> bool {
          scheme == "ftp" || scheme == "ftps"
      }
      
      async fn download(
          &self,
          uri: &str,
      ) -> Result<(u64, Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>)> {
          let url = Url::parse(uri).context("Failed to parse FTP URL")?;
          
          let host = url.host_str()
              .ok_or_else(|| anyhow::anyhow!("FTP URL missing host"))?;
          let port = url.port().unwrap_or(21);
          let path = url.path();
          
          let mut ftp_stream = AsyncFtpStream::connect((host, port))
              .await
              .context("Failed to connect to FTP server")?;
          
          // Handle authentication from URL
          if !url.username().is_empty() {
              let password = url.password().unwrap_or("");
              ftp_stream.login(url.username(), password)
                  .await
                  .context("FTP login failed")?;
          } else {
              ftp_stream.login("anonymous", "anonymous@")
                  .await
                  .context("Anonymous FTP login failed")?;
          }
          
          // Get file size
          let size = ftp_stream.size(path)
              .await
              .context("Failed to get file size")?
              .ok_or_else(|| anyhow::anyhow!("FTP server didn't return file size"))?;
          
          // Download file
          let reader = ftp_stream.retr_as_stream(path)
              .await
              .context("Failed to retrieve file")?
              .compat();
          
          // Convert AsyncRead to Stream of chunks
          let chunks = tokio_util::io::ReaderStream::new(reader)
              .map(|result| result
                  .map(|bytes| bytes.freeze())
                  .map_err(|e| anyhow::anyhow!("FTP read error: {}", e))
              );
          
          Ok((size as u64, Box::pin(chunks)))
      }
      
      async fn upload(
          &self,
          _uri: &str,
          _data: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
      ) -> Result<()> {
          Err(anyhow::anyhow!("FTP upload not implemented"))
      }
  }
  ```
- Success: FTP downloads work with embedded credentials

### 5. Create Handler Registry

- File: `/client/rpc/src/remote_file/registry.rs` (create new)
- Operation: Create registry for scheme-based handler selection
- Details:
  ```rust
  use super::{RemoteFileHandler, local::LocalFileHandler, http::HttpFileHandler, ftp::FtpFileHandler};
  use std::sync::Arc;
  use anyhow::{Result, Context};
  use url::Url;
  
  pub struct HandlerRegistry {
      handlers: Vec<Arc<dyn RemoteFileHandler>>,
  }
  
  impl HandlerRegistry {
      pub fn new() -> Self {
          Self {
              handlers: vec![
                  Arc::new(LocalFileHandler),
                  Arc::new(HttpFileHandler::new()),
                  Arc::new(FtpFileHandler),
              ],
          }
      }
      
      pub fn get_handler(&self, uri: &str) -> Result<Arc<dyn RemoteFileHandler>> {
          let scheme = if uri.contains("://") {
              Url::parse(uri)
                  .context("Failed to parse URI")?
                  .scheme()
                  .to_string()
          } else {
              // No scheme means local file
              String::new()
          };
          
          self.handlers
              .iter()
              .find(|handler| handler.supports_scheme(&scheme))
              .cloned()
              .ok_or_else(|| anyhow::anyhow!("No handler found for scheme: {}", scheme))
      }
  }
  
  impl Default for HandlerRegistry {
      fn default() -> Self {
          Self::new()
      }
  }
  ```
- Success: Registry correctly selects handlers based on URI scheme

### 6. Update RPC Module Dependencies

- File: `/client/rpc/src/lib.rs`
- Operation: Add module declaration and imports (after line 22, before other imports)
- Details:
  ```rust
  mod remote_file;
  use remote_file::HandlerRegistry;
  use futures::StreamExt;
  ```
- Success: Module is included in compilation

### 7. Update loadFileInStorage Method

- File: `/client/rpc/src/lib.rs`
- Operation: Modify load_file_in_storage to use handler registry (lines 323-410)
- Details:
  - Replace the file reading section (approximately lines 330-375):
  ```rust
  // Create handler registry
  let registry = HandlerRegistry::new();
  let handler = registry.get_handler(&file_path)
      .map_err(|e| into_rpc_error(format!("Failed to get handler: {}", e)))?;
  
  // Download file using appropriate handler
  let (file_size, mut stream) = handler.download(&file_path).await
      .map_err(|e| into_rpc_error(format!("Failed to download file: {}", e)))?;
  
  // Use the existing trie and chunk processing logic
  let mut file_data_trie = FileDataTrieBuilder::new();
  let mut chunk_id = ChunkId::new(0u64);
  
  while let Some(chunk_result) = stream.next().await {
      let chunk_data = chunk_result
          .map_err(|e| into_rpc_error(format!("Stream error: {}", e)))?;
      
      // Convert bytes to Vec<u8> for compatibility
      let chunk_vec = chunk_data.to_vec();
      
      // Insert chunk into trie
      let proven_leaves = file_data_trie
          .write_chunk(&chunk_id, &chunk_vec, &proved_keys)
          .map_err(|e| into_rpc_error(format!("Failed to write chunk: {}", e)))?;
      
      storage
          .write()
          .await
          .insert_chunks_into_bag(&file_key, &[(chunk_id, chunk_vec)])
          .map_err(|e| into_rpc_error(e))?;
      
      chunk_id = chunk_id.checked_add(1).ok_or_else(|| {
          into_rpc_error("Chunk ID overflow".to_string())
      })?;
  }
  
  // Continue with existing metadata generation...
  ```
- Success: Remote URLs are processed correctly

### 8. Update saveFileToDisk Method

- File: `/client/rpc/src/lib.rs`
- Operation: Modify save_file_to_disk to support local upload only (lines 452-507)
- Details:
  - Add at the beginning of the function:
  ```rust
  // Create handler registry
  let registry = HandlerRegistry::new();
  let handler = registry.get_handler(&file_path)
      .map_err(|e| into_rpc_error(format!("Failed to get handler: {}", e)))?;
  
  // Check if handler supports upload
  if !handler.supports_upload() {
      return Err(into_rpc_error(
          "Upload not supported for this protocol. Only local file paths are supported for saving."
      ));
  }
  
  // Existing file retrieval logic...
  let file_storage = storage.read().await;
  // ... (keep existing metadata and chunk reading logic)
  
  // Create a stream from chunks
  let chunk_stream = futures::stream::iter(chunks.into_iter())
      .map(|chunk| Ok(bytes::Bytes::from(chunk)));
  
  // Upload using handler
  handler.upload(&file_path, Box::pin(chunk_stream)).await
      .map_err(|e| into_rpc_error(format!("Failed to save file: {}", e)))?;
  ```
- Success: Local saves work, remote saves fail with clear error

### 9. Add Integration Tests

- File: `/client/rpc/src/remote_file/mod.rs`
- Operation: Add test module at end of file
- Details:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      
      #[tokio::test]
      async fn test_local_handler_scheme_detection() {
          let handler = local::LocalFileHandler;
          assert!(handler.supports_scheme(""));
          assert!(handler.supports_scheme("file"));
          assert!(!handler.supports_scheme("http"));
      }
      
      #[tokio::test]
      async fn test_http_handler_scheme_detection() {
          let handler = http::HttpFileHandler::new();
          assert!(handler.supports_scheme("http"));
          assert!(handler.supports_scheme("https"));
          assert!(!handler.supports_scheme("ftp"));
      }
      
      #[tokio::test]
      async fn test_registry_handler_selection() {
          let registry = HandlerRegistry::new();
          
          // Test local file
          assert!(registry.get_handler("/path/to/file").is_ok());
          
          // Test HTTP
          assert!(registry.get_handler("http://example.com/file").is_ok());
          
          // Test unsupported
          assert!(registry.get_handler("sftp://example.com/file").is_err());
      }
  }
  ```
- Success: Tests pass

## Testing Strategy

- [ ] Unit tests for each handler's scheme detection
- [ ] Integration test with local file paths (regression)
- [ ] Integration test with HTTP URL (httpbin.org)
- [ ] Integration test with FTP URL (public test server)
- [ ] Error handling tests for invalid URLs
- [ ] Network timeout simulation tests
- [ ] Large file streaming tests (memory usage verification)

## Rollback Plan

1. Remove `/client/rpc/src/remote_file/` directory
2. Remove module declaration from `/client/rpc/src/lib.rs`
3. Revert changes to loadFileInStorage and saveFileToDisk
4. Remove new dependencies from Cargo.toml