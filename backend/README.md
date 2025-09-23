# StorageHub Backend

This backend provides a REST API for interacting with a StorageHub MSP node. It exposes features for authentication, MSP discovery and health, bucket and file navigation, file transfer (upload/download), distribution triggers, and payment stream queries. Under the hood it integrates with the StorageHub client RPC for calls and with the Postgres DB of the indexer for queries.

## Architecture

- Entry (HTTP): axum-based server exposes REST endpoints; handlers in `lib/src/api/handlers.rs`.
- Services layer: business logic and orchestration in `lib/src/services/*`.
  - `MspService`: MSP-facing operations (info/stats, buckets, files, distribution, upload/download).
  - `HealthService`: probes storage, DB, and RPC health.
  - `AuthService`: JWT issuance/verification for endpoints requiring auth.
- Data layer: abstractions around dependencies in `lib/src/data/*`.
  - Storage: `BoxedStorage` with in-memory implementation for tests.
  - DB: `DBClient` with a mock repository for tests.
  - RPC: `StorageHubRpcClient` on top of a jsonrpsee WebSocket client.
    - Method calls are forwarded directly with jsonrpsee `ToRpcParams` (tuples map to JSON-RPC arrays).
- Client node integration: the backend communicates with the StorageHub MSP client RPC for file- and storage-related operations (e.g., uploading batches, downloading files) without managing a P2P connection directly.

### Core flows (high-level)

- Authentication: issue/verify/refresh JWTs for endpoint access.
- MSP discovery & health: expose node info, stats, value props, and composite health.
- Bucket navigation: list user buckets, fetch bucket details, list files by path.
- File operations:
  - Upload: stream multipart, validate metadata, chunk into trie, batch proofs, forward via client RPC.
  - Download: complex flow involving MSP client RPC coordination (see detailed flow below).
  - Metadata: query file info for display and validation.
- Distribution: trigger a distribution flow for a file (mocked response).
- Payments: query payment stream state for the authenticated user.

### File Download Flow (Internal Coordination)

The file download mechanism involves a sophisticated coordination between the backend and the MSP Substrate client through an internal upload endpoint:

1. **SDK Request**: The SDK calls the `/download/{file_key}` route of the backend with a file key.

2. **Internal URL Generation**: The `download_by_key` handler calls the `get_file_from_key` function of the MSP service, which:
   - Creates a temporary file path: `/tmp/uploads/{file_key}`
   - Generates an internal callback URL: `{msp_callback_url}/internal/uploads/{file_key}`

3. **RPC Call to MSP Client**: The MSP service calls the `saveFileToDisk` RPC method on the StorageHub client with:
   - The file key to retrieve from the client's file storage
   - The internal callback URL where the client should upload the file

4. **Client-to-Backend Upload**: The MSP client RPC method:
   - Retrieves the file from its internal file storage
   - Streams the file data to the backend's internal upload endpoint (`PUT /internal/uploads/{file_key}`)
   - Returns file metadata if successful

5. **Internal Upload Handler**: The `internal_upload_by_key` handler:
   - Validates the file key format (hex string)
   - Creates the `/tmp/uploads` directory if needed
   - Writes the received file data to `/tmp/uploads/{file_key}`
   - Returns success/error status

6. **File Streaming Response**: If the RPC call succeeds:
   - The backend opens the temporary file from `/tmp/uploads/{file_key}`
   - Creates a file stream response for the client
   - Immediately unlinks the temporary file (on Unix systems) while keeping the file descriptor open for streaming
   - Returns the file stream to the original SDK request

This design allows the backend to act as a bridge between external SDK requests and the internal MSP client file storage, enabling secure file retrieval without exposing the client's internal storage directly.

**Note**: The internal upload endpoint (`/internal/uploads/{file_key}`) should only be accessible by the MSP client RPC and not exposed to external clients.

## Endpoints

Auth

- POST `/auth/nonce` -> request challenge (address, chain_id) -> returns nonce
- POST `/auth/verify` -> verify signature and issue token
- POST `/auth/refresh` -> refresh toke
- POST `/auth/logout` -> revoke token
- GET `/auth/profile` -> returns profile

MSP Info

- GET `/info` -> node client/version, msp_id, multiaddresses, etc.
- GET `/stats` -> capacity and basic counters
- GET `/value-props` -> value proposition list
- GET `/health` -> composite health status (storage, RPC, DB, etc.)

Buckets

- GET `/buckets` (auth) -> list buckets for current user
- GET `/buckets/:bucketId` (auth) -> bucket details
- GET `/buckets/:bucketId/files?path=/sub/dir` (auth) -> list of immediate children under path

Files

- GET `/buckets/:bucketId/info/:fileKey` (auth) -> file info
- GET `/buckets/:bucketId/download/path/:file_location` (auth) -> download by logical location
- GET `/download/:fileKey` -> download by file key (triggers internal coordination flow)
- PUT `/buckets/:bucketId/upload/:fileKey` (auth, multipart)
  - Fields: `file_metadata` (SCALE-encoded), `file` (binary stream)
  - Streams, validates, batches, and forwards via client RPC

Internal (MSP Client Only)

- PUT `/internal/uploads/:fileKey` -> temporary upload endpoint used by MSP client RPC
  - Receives file data from the StorageHub client's `saveFileToDisk` RPC call
  - Writes file to `/tmp/uploads/{file_key}` for subsequent streaming to SDK clients (this is temporary, the file should be directly streamed to the SDK client)
  - **Security Note**: This endpoint should only be accessible by the local MSP client, not external clients

Distribution

- POST `/buckets/:bucketId/files/:fileKey/distribute` (auth) -> triggers distribution to BSPs

Payments

- GET `/payment_streams` (auth) -> returns payment stream information of the current user

Notes

- Exact router setup is in the server crate (outside this library). Handlers documented here correspond to functions in `lib/src/api/handlers.rs`.

## Endpoint examples

Assume backend is reachable at `$BACKEND_URL` (e.g., `http://localhost:8080`) and you have a valid `Bearer $TOKEN` for auth endpoints.

Auth

```bash
curl -sX POST "$BACKEND_URL/auth/nonce" \
  -H 'Content-Type: application/json' \
  -d '{"address":"0x...","chain_id":1284}'

curl -sX POST "$BACKEND_URL/auth/verify" \
  -H 'Content-Type: application/json' \
  -d '{"message":"...","signature":"0x..."}'

curl -sX POST "$BACKEND_URL/auth/refresh" -H "Authorization: Bearer $TOKEN"
curl -sX POST "$BACKEND_URL/auth/logout"  -H "Authorization: Bearer $TOKEN"
curl -s "$BACKEND_URL/auth/profile"       -H "Authorization: Bearer $TOKEN"
```

MSP info & health

```bash
curl -s "$BACKEND_URL/info"
curl -s "$BACKEND_URL/stats"
curl -s "$BACKEND_URL/value-props"
curl -s "$BACKEND_URL/health"
```

Buckets

```bash
curl -s "$BACKEND_URL/buckets" -H "Authorization: Bearer $TOKEN"
curl -s "$BACKEND_URL/buckets/<bucketId>" -H "Authorization: Bearer $TOKEN"
curl -s "$BACKEND_URL/buckets/<bucketId>/files?path=sub/dir" -H "Authorization: Bearer $TOKEN"
```

Files (metadata & download)

```bash
curl -s "$BACKEND_URL/buckets/<bucketId>/info/<fileKey>" -H "Authorization: Bearer $TOKEN"
curl -s "$BACKEND_URL/download/<fileKey>" -H "Authorization: Bearer $TOKEN" -o out.bin

# Internal endpoint (MSP client only - not for external use)
# This is called automatically by the MSP client during the download flow
# curl -X PUT "$BACKEND_URL/internal/uploads/<fileKey>" --data-binary @file.bin
```

File upload (multipart)

```bash
# file_metadata.bin is SCALE-encoded FileMetadata bytes
curl -sX PUT "$BACKEND_URL/buckets/<bucketId>/upload/<fileKey>" \
  -H "Authorization: Bearer $TOKEN" \
  -F file=@/path/to/file.bin \
  -F file_metadata=@/path/to/file_metadata.bin;type=application/octet-stream
```

Distribute

```bash
curl -sX POST "$BACKEND_URL/buckets/<bucketId>/files/<fileKey>/distribute" \
  -H "Authorization: Bearer $TOKEN"
```

Payments

```bash
curl -s "$BACKEND_URL/payment_stream" -H "Authorization: Bearer $TOKEN"
```

## Under the hood

- JSON-RPC client: `StorageHubRpcClient` wraps `jsonrpsee` WS client with direct `request(method, params)` calls.
- Params serialization uses `ToRpcParams` (tuples map to JSON-RPC arrays; use `rpc_params![]` for empty params).
- MSP client RPCs used include:
  - `storagehubclient_saveFileToDisk`: Downloads files from client storage to backend via internal upload endpoint
  - `storagehubclient_isFileKeyExpected`: Checks if the MSP client expects a specific file key
  - `storagehubclient_receiveBackendFileChunks`: Uploads file chunks and proofs to the MSP client

## Build

Prereqs: Rust toolchain, pnpm (for cross-build script), Docker (for container build).

- Standard build (Linux/CI or non-macOS):
  - From repo root:
    - `cargo build -p sh-msp-backend --release`

- macOS cross build (to Linux target used in Docker):
  - From repo root:
    - `cd test`
    - `pnpm install` (first time)
    - `pnpm crossbuild:mac:backend`

Artifacts:

- Native: `backend/target/release/libsh_msp_backend_lib.rlib` (library) and any binaries in backend workspace.
- Cross-built: artifacts placed under `build/sh-msp-backend` per the script.

## Dockerize

- From repo root:
  - `cd test`
  - `pnpm docker:build:backend`

This uses the prebuilt backend artifact (via cross-build on macOS or cargo build on Linux) and `docker/storage-hub-msp-backend.Dockerfile` to produce the backend image.

## Testing

- Library tests with mocks:
  - `cargo test -p sh-msp-backend-lib --features mocks`

Coverage (mocked):

- `MspService`: info/stats/value-props/list buckets/get files/get file info/distribute/payment stream/upload.
- RPC layer: mock connection covers error modes and basic request/response.

## Directory layout (backend)

- `backend/lib/src/api/handlers.rs` — HTTP handlers
- `backend/lib/src/services/*` — business logic (msp, health, auth)
- `backend/lib/src/data/*` — storage/DB/RPC abstractions
- `backend/lib/src/models/*` — API models
- `backend/lib/src/lib.rs` — library entry
- `docker/storage-hub-msp-backend.Dockerfile` — container build
