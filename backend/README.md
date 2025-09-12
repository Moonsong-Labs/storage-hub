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
  - Download: endpoints to fetch by location or key (mocked payloads in current implementation).
  - Metadata: query file info for display and validation.
- Distribution: trigger a distribution flow for a file (mocked response).
- Payments: query payment stream state for the authenticated user.

## Endpoints

Auth

- POST `/auth/nonce` -> request challenge (address, chain_id) -> returns nonce
- POST `/auth/verify` -> verify signature and issue token
- POST `/auth/refresh` -> refresh toke
- POST `/auth/logout` -> revoke token
- GET `/auth/profile` -> returns profile

MSP Info

- GET `/msp/info` -> node client/version, msp_id, multiaddresses, etc.
- GET `/msp/stats` -> capacity and basic counters
- GET `/msp/value-props` -> mocked value proposition list
- GET `/msp/health` -> composite health status (storage, RPC, DB, etc.)

Buckets

- GET `/buckets` (auth) -> list buckets for current user
- GET `/buckets/:bucketId` (auth) -> bucket details
- GET `/buckets/:bucketId/files?path=/sub/dir` (auth) -> list of immediate children under path

Files

- GET `/buckets/:bucketId/files/:fileKey/info` (auth) -> file info
- GET `/buckets/:bucketId/files/location/:location/download` (auth) -> download by logical location
- GET `/buckets/:bucketId/files/:fileKey/download` (auth) -> download by file key
- POST `/buckets/:bucketId/files/:fileKey/upload` (auth, multipart)
  - Fields: `file_metadata` (SCALE-encoded), `file` (binary stream)
  - Streams, validates, batches, and forwards via client RPC

Distribution

- POST `/buckets/:bucketId/files/:fileKey/distribute` (auth) -> triggers distribution to BSPs

Payments

- GET `/payments/stream` (auth) -> returns payment stream status for the current user

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
curl -s "$BACKEND_URL/msp/info"
curl -s "$BACKEND_URL/msp/stats"
curl -s "$BACKEND_URL/msp/value-props"
curl -s "$BACKEND_URL/msp/health"
```

Buckets

```bash
curl -s "$BACKEND_URL/buckets" -H "Authorization: Bearer $TOKEN"
curl -s "$BACKEND_URL/buckets/<bucketId>" -H "Authorization: Bearer $TOKEN"
curl -s "$BACKEND_URL/buckets/<bucketId>/files?path=sub/dir" -H "Authorization: Bearer $TOKEN"
```

Files (metadata & download)

```bash
curl -s "$BACKEND_URL/buckets/<bucketId>/files/<fileKey>/info" -H "Authorization: Bearer $TOKEN"
curl -s "$BACKEND_URL/buckets/<bucketId>/files/location/<location>/download" -H "Authorization: Bearer $TOKEN" -o out.bin
curl -s "$BACKEND_URL/buckets/<bucketId>/files/<fileKey>/download" -H "Authorization: Bearer $TOKEN" -o out.bin
```

File upload (multipart)

```bash
# file_metadata.bin is SCALE-encoded FileMetadata bytes
curl -sX POST "$BACKEND_URL/buckets/<bucketId>/files/<fileKey>/upload" \
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
curl -s "$BACKEND_URL/payments/stream" -H "Authorization: Bearer $TOKEN"
```

## Under the hood

- JSON-RPC client: `StorageHubRpcClient` wraps `jsonrpsee` WS client with direct `request(method, params)` calls.
- Params serialization uses `ToRpcParams` (tuples map to JSON-RPC arrays; use `rpc_params![]` for empty params).
- MSP client RPCs used include file/storage helpers (e.g., upload to peer) and may expand as features evolve.

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

This uses the prebuilt backend artifact (via cross-build on macOS or cargo build on Linux) and `backend/Dockerfile` to produce the backend image.

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
- `backend/Dockerfile` — container build
