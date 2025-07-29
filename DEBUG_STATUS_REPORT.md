# HTTP Integration Test Debug Status Report

## Current Issue
The HTTP integration tests for StorageHub are failing with a fingerprint mismatch when downloading files from copyparty via HTTP, while FTP downloads work correctly.

### Expected vs Actual
- **Expected fingerprint**: `0x34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970`
- **Actual fingerprint**: `0xe8da665e9c9b985dcad3b34f494df9252617c197df59cbbdb1e80a23671394e5`
- **File**: `adolphus.jpg` (416,400 bytes)

## Investigation Findings

### 1. HTTP Implementation Analysis
- The HTTP handler uses `BufReader` with 8KB buffer size (matching FTP's effective buffering)
- The chunking logic in `load_file_in_storage` correctly reads exactly 1024 bytes per chunk
- Unit tests with mock HTTP server pass correctly and produce the expected fingerprint

### 2. FTP Comparison
- FTP implementation works correctly in integration tests
- FTP uses channel-based buffering (8KB chunks) through `FtpStreamReader`
- Both HTTP and FTP effectively buffer data, so buffering itself is not the issue

### 3. Copyparty Server Testing
- Created a standalone test (`test_fingerprint_copyparty`) that downloads from a locally running copyparty
- This test **passes successfully** and produces the correct fingerprint
- Verified copyparty serves the correct file:
  - SHA256: `739fb97f7c2b8e7f192b608722a60dc67ee0797c85ff1ea849c41333a40194f2`
  - Size: 416,400 bytes
  - Fingerprint matches expected value when downloaded outside of integration test environment

### 4. Key Observations
- The wrong fingerprint (`0xe8da665...`) is consistent across test runs (not random)
- The issue only occurs in the integration test environment
- Direct HTTP download from copyparty works correctly
- The same code produces different results in different environments

## Current Hypothesis
The issue appears to be environment-specific, occurring only when:
1. The StorageHub node runs inside a Docker container
2. It downloads from copyparty also running in a Docker container
3. Via Docker's internal network

Possible causes:
- Docker networking layer modifying the HTTP stream
- Different copyparty behavior when accessed via container hostname
- HTTP client behavior differences in containerized environment
- Potential encoding/compression issues in the Docker network path

## Debug Code Added

### 1. Integration Test Debug (load-file-storage.test.ts)
- Added SHA256 verification from within the user container
- Added content-length header verification
- Added detailed logging of fingerprint calculation results

### 2. RPC Implementation Debug (lib.rs)
- Added logging of chunk details (size, first bytes)
- Added total chunks and bytes tracking
- Added URL logging for verification

## Next Steps

1. **Run the enhanced integration test** to collect:
   - SHA256 of file as seen from user container
   - Content-Length headers
   - Detailed chunk information from RPC logs
   - Actual vs expected fingerprint values

2. **Compare container logs** to see:
   - What URL is actually being used
   - How many chunks are read
   - First bytes of each chunk
   - Total bytes processed

## Resolution
The issue has been resolved. The root cause was identified and fixed in commit 545a652.

### Root Cause
The HTTP stream was returning data in variable chunk sizes (not always filling the full buffer), while the FTP implementation always read full chunks. The fingerprint calculation depends on exact chunk boundaries, so different read patterns produced different fingerprints even though the file content was identical.

### The Fix
Modified `load_file_in_storage` in `client/rpc/src/lib.rs` to ensure it always reads exactly `FILE_CHUNK_SIZE` bytes per chunk (except the last one). This ensures consistent fingerprints regardless of how the underlying stream returns data.

### Key Code Change
Instead of accepting whatever bytes the stream returns in a single read, the code now loops until it fills each chunk completely:
```rust
// Keep reading until we fill the chunk or hit EOF
while offset < FILE_CHUNK_SIZE as usize {
    match stream.read(&mut chunk[offset..]).await {
        // Continue reading until chunk is full
    }
}
```

### Verification
After rebuilding Docker images with this fix, the HTTP integration tests now pass with the correct fingerprint:
- Expected: `0x34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970`
- Actual: `0x34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970` ✓

## Important Development Guidelines

### 1. Building and Testing
**ALWAYS build/test the entire workspace from the root directory.** Using package-specific flags like `-p shc-rpc` or attempting to test single packages leads to unrelated build failures (substrate compilation errors). Always run:
```bash
# From workspace root
cargo build
cargo test
# NOT: cargo test -p shc-rpc
```

### 2. Integration Test Requirements
The integration tests rely on locally-built Docker images that must be reconstructed when the node code is modified:
```bash
cd test
pnpm i
pnpm crossbuild:mac    # Cross-compile for Docker
pnpm docker:build      # Build Docker images
```

### 3. Preserving Progress
**When a key finding is discovered, immediately commit the current changes** with a descriptive message that includes the finding. This prevents loss of valuable debugging information:
```bash
git add -A
git commit -m "fix: add HTTP debugging - found fingerprint mismatch in Docker environment"
```
