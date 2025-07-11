# Remote File Handler Testing Guide

This guide explains how to run tests for the remote file handling functionality in StorageHub RPC.

## Test Structure

The remote file module includes comprehensive tests organized into several categories:

### 1. Unit Tests

Located in each handler module (`http.rs`, `ftp.rs`, `local.rs`, `factory.rs`):

- **Factory Tests**: Test the factory's ability to create correct handlers for each URL scheme
- **URL Parsing Tests**: Verify URL validation and scheme detection
- **Error Handling Tests**: Ensure proper error responses for invalid inputs
- **Handler-specific Tests**: Test individual handler functionality with mocks

### 2. Integration Tests

Located in `tests.rs`:

- **Factory Integration**: Tests the complete factory workflow
- **Config Tests**: Verify configuration handling
- **Mock Handler Tests**: Test the trait implementation with a mock handler
- **External Service Tests**: Tests that require real external services (marked with `#[ignore]`)

### 3. RPC Integration Tests

Located in `tests/remote_file_rpc_tests.rs`:

- Tests the integration between remote file handlers and RPC methods
- Validates request/response structures
- Tests error propagation through the RPC layer

## Running Tests

### Run All Unit Tests
```bash
cargo test -p shc-rpc --lib remote_file
```

### Run Factory Tests Only
```bash
cargo test -p shc-rpc --lib remote_file::factory::tests
```

### Run HTTP Handler Tests
```bash
cargo test -p shc-rpc --lib remote_file::http::tests
```

### Run FTP Handler Tests
```bash
cargo test -p shc-rpc --lib remote_file::ftp::tests
```

### Run Local File Handler Tests
```bash
cargo test -p shc-rpc --lib remote_file::local::tests
```

### Run Integration Tests
```bash
cargo test -p shc-rpc --lib remote_file::tests
```

### Run Tests Requiring External Services

Some tests require external services and are marked with `#[ignore]`. To run these:

```bash
# Run all ignored tests
cargo test -p shc-rpc --lib -- --ignored

# Run only FTP tests that require external server
cargo test -p shc-rpc --lib remote_file::ftp::tests -- --ignored
```

## External Service Tests

### FTP Tests

FTP tests use the public test server at `ftp://test.rebex.net`. These tests are ignored by default because they:
- Require internet connectivity
- Depend on external service availability
- May be slower than unit tests

To run FTP integration tests:
```bash
cargo test -p shc-rpc --lib test_ftp -- --ignored
```

### HTTP Tests

HTTP tests use the `mockito` library for mocking, but some integration tests might use:
- `httpbin.org` for testing various HTTP scenarios
- Local web servers for testing large files

## Environment Variables

You can configure test behavior with these environment variables:

- `TEST_FTP_SERVER`: Override the default FTP test server (default: `ftp://test.rebex.net`)
- `TEST_HTTP_SERVER`: Override the default HTTP test server (default: `https://httpbin.org`)
- `TEST_FILE_SIZE`: Maximum file size to use in tests (default: 1MB)

Example:
```bash
TEST_FTP_SERVER=ftp://mytest.server.com cargo test -- --ignored
```

## Writing New Tests

When adding new tests:

1. **Unit Tests**: Add to the appropriate handler module with mocks
2. **Integration Tests**: Add to `tests.rs` for cross-handler functionality
3. **External Tests**: Mark with `#[ignore]` and document requirements
4. **RPC Tests**: Add to `tests/remote_file_rpc_tests.rs` for RPC integration

### Test Template

```rust
#[tokio::test]
async fn test_new_functionality() {
    // Arrange
    let config = RemoteFileConfig::default();
    let url = Url::parse("https://example.com/file.txt").unwrap();
    
    // Act
    let handler = RemoteFileHandlerFactory::create(&url, config).unwrap();
    let result = handler.fetch_metadata(&url).await;
    
    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "Requires external service"]
async fn test_with_real_service() {
    // Test that requires external connectivity
}
```

## Coverage

To generate test coverage reports:

```bash
# Install tarpaulin if not already installed
cargo install cargo-tarpaulin

# Run coverage for remote file module
cargo tarpaulin -p shc-rpc --lib -- remote_file
```

## Troubleshooting

### Common Issues

1. **Compilation Errors**: Ensure all dependencies are up to date
   ```bash
   cargo update
   ```

2. **Test Timeouts**: Some external service tests may timeout. Increase timeout with:
   ```bash
   RUST_TEST_THREADS=1 cargo test -- --test-threads=1
   ```

3. **FTP Test Failures**: The public test FTP server may be down. Check connectivity:
   ```bash
   curl -I ftp://test.rebex.net/readme.txt
   ```

4. **Mock Server Issues**: Ensure mockito server is properly initialized in tests

## CI/CD Integration

For CI pipelines, exclude external service tests:

```yaml
# Run only unit tests in CI
- run: cargo test -p shc-rpc --lib remote_file

# Run external tests separately (optional, may fail)
- run: cargo test -p shc-rpc --lib remote_file -- --ignored
  continue-on-error: true
```