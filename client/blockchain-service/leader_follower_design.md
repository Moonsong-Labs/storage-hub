### Blockchain Service Leader/Follower Design

This document describes the design for running multiple instances of the same MSP/BSP node concurrently using a **leader/follower** model coordinated via Postgres and advisory locks.

The goal is:

- **Exactly one leader per provider account** (MSP/BSP) at any time.
- **Zero competition on nonces / transaction submission** (only the leader sends extrinsics).
- **Followers stay up to date with chain state (e.g. Forest roots) and file data**, so that they can take over as leader quickly and safely when promoted.

No code is implemented yet; this is the implementation plan.

---

### 1. Roles and invariants

- **Node roles**

  - `Leader`:
    - The only instance allowed to:
      - Construct and submit extrinsics (including gap-filling transactions).
      - Own RPC `author_submitAndWatchExtrinsic` subscriptions.
      - Drive nonce gap detection and gap-filling.
      - Perform DB cleanup of old pending transactions.
      - Emit internal Blockchain Service events that trigger MSP/BSP tasks (capacity management, proofs, storage requests, etc.).
  - `Follower`:
    - Never submits extrinsics.
    - Never manages nonces or gap-filling.
    - Does **not** emit internal events that trigger tasks while it is a follower.
    - Keeps its local view of chain state (e.g. Forest roots) in sync via the normal Blockchain Service read path.
    - Keeps its file data in sync with the leader (e.g. by the leader pushing chunks to followers), so that it is ready to take over.
  - `Standalone`:
    - Pending-tx DB is disabled.
    - Behaves like a single-instance deployment: runs tasks and sends extrinsics, but without coordinating pending transactions via Postgres.

- **Invariants**
  - At most **one leader per HA group** (MSP/BSP account) at any time.
  - Followers **must not** promote themselves to leader unless they have successfully acquired the Postgres advisory lock.
  - Followers are **heavily dependent on the DB** for any shared state; if DB is degraded, they cannot assume leadership.

---

### 2. Leader election via Postgres advisory locks

We will use **Postgres session advisory locks** for simple leader election.

- **Lock key**

  - For the first iteration, use a **hardcoded 64-bit key** (e.g. a constant `i64`) shared by all instances in the same HA group.
  - All instances that:
    - Share the same Postgres DB, and
    - Share the same keystore / provider account,
      will contend on this single advisory lock.
  - Future work: derive the key from a stable identifier (e.g. provider ID, account ID) when supporting multiple providers per DB.

- **Dedicated leadership connection (TLS compatible)**

  - Advisory locks are **per session / per connection**. Pooled connections are not suitable for “hold lock for process lifetime”.
  - We will create a **non-pooled dedicated Postgres connection** that:
    - Is created at `BlockchainService` startup.
    - Is kept open for the lifetime of the process (or until shutdown).
    - Uses the **same TLS story as the async pool**:
      - TLS via `tokio-postgres-rustls` with the same environment-driven config:
        - `SH_DB_TLS_INSECURE` to disable verification (for local/dev).
        - `SH_DB_TLS_CA_FILE` to load a custom CA bundle.
        - Otherwise, platform verifier / system trust store.
    - If this connection drops, the session ends and the advisory lock is automatically released.

- **Election algorithm**

  - At startup, if the pending-tx DB is configured:
    1. Create the dedicated leadership connection (TLS as above).
    2. Execute `SELECT pg_try_advisory_lock(<HARDCODED_KEY>)`:
       - `true` → node becomes **Leader**.
       - `false` → node becomes **Follower**.
  - At runtime:
    - **Leader**:
      - Assumes it still holds the lock as long as the leadership connection is alive.
      - On clean shutdown, it may release the lock explicitly, but this is optional.
    - **Followers**:
      - Run a periodic task (with small random jitter) that calls `pg_try_advisory_lock` on the leadership connection:
        - If it ever returns `true`, the follower **promotes** to leader.
      - Advisory locks alone are sufficient; no extra “heartbeat” logic is required for correctness.

- **No role mode for now**
  - We do **not** implement a `RoleMode`/`ForceLeader`/`ForceFollower` mechanism yet.
  - All nodes use this automatic advisory-lock-based selection.

---

### 3. DB crate extensions (`shc-blockchain-service-db`)

We extend the DB crate to support leadership and richer notifications.

#### 3.1 Leadership helpers

- New module (e.g. `leadership.rs`) providing:
  - `fn open_leadership_connection(database_url: &str) -> Result<tokio_postgres::Client, DbSetupError>`:
    - Uses **TLS-compatible** setup mirroring `make_rustls_config_from_env`.
    - Creates a dedicated, non-pooled connection suitable for advisory locks.
  - `async fn try_acquire_leadership(client: &tokio_postgres::Client, key: i64) -> Result<bool, DbSetupError>`:
    - Executes `SELECT pg_try_advisory_lock($1)` and returns `Ok(true)` if lock obtained.
  - (Optional) `async fn release_leadership(client: &tokio_postgres::Client, key: i64)`:
    - Executes `SELECT pg_advisory_unlock($1)` for graceful shutdown, though not strictly required.

These helpers **must** use the same TLS configuration logic as `setup_db_pool`, so that leadership works in environments where TLS is mandatory.

#### 3.2 NOTIFY on updates as well as inserts

The existing migration already defines:

- Table `pending_transactions`.
- Index on `(hash)`.
- Trigger/function `notify_pending_tx_new` that issues:
  - `pg_notify('pending_tx_new', json_build_object(...))` on **INSERT**.

We need to:

- Ensure followers can also see **state transitions**, not just new rows.
- Implement **Option B** from the earlier design: enrich DB / NOTIFY payloads with enough data for followers to reconstruct accurate `TransactionStatus` values, including block hashes where applicable.

Plan:

- Extend the DB layer so that **updates also generate NOTIFYs**:

  - EITHER via:
    - A new trigger `notify_pending_tx_update` on `UPDATE` of `pending_transactions`, emitting `pg_notify('pending_tx_update', json_build_object(...))`.
  - OR via:
    - Explicit `SELECT pg_notify(...)` calls inside `PendingTxStore::update_state`, after a successful `UPDATE`.
  - The choice can depend on ergonomics; a trigger keeps DB-side behaviour centralised, explicit calls keep it Rust-local.

- NOTIFY payload contents:
  - For both insert and update notifications, we want a JSON object with at least:
    - `account_id` (hex-encoded).
    - `nonce`.
    - `hash` (hex-encoded).
    - `state` (string as stored).
    - `creator_id`.
    - `updated_at`.
  - For updates corresponding to non-terminal Substrate statuses that carry block info, we should also include:
    - `block_hash` (hex-encoded), when known.
  - This likely requires extending `PendingTxStore::update_state` to accept an optional block hash and/or other status metadata, and persisting it either:
    - In new columns, or
    - Only in the NOTIFY payload (if we do not need it stored long-term).

Exact schema changes for Option B need to be designed carefully to avoid over-complicating the table, but the intent is clear: **enough information for followers to reconstruct realistic `TransactionStatus` values.**

#### 3.3 LISTEN helper

- Add a small helper in the DB crate to create a **LISTEN loop**:
  - This can use another dedicated `tokio_postgres` client (with TLS) that:
    - Executes `LISTEN pending_tx_new;` and `LISTEN pending_tx_update;`.
    - Yields notification payloads via a Tokio stream or callback API.
  - This helper should be generic enough that `blockchain-service` can plug its own task around it.

---

### 4. Heartbeat for monitoring (optional, for observability)

Although advisory locks are sufficient for correctness, a simple heartbeat makes operational monitoring easier.

- **Schema**

  - A small table, for example:

    - `leader_heartbeat`:
      - `provider_id` (TEXT or BYTEA) – logical ID of MSP/BSP (may initially be a constant / placeholder).
      - `instance_id` (TEXT) – logical ID of this node instance (e.g. from env var).
      - `last_seen TIMESTAMPTZ`.
      - Primary key on `(provider_id, instance_id)` or `(provider_id)`, depending on how we want to aggregate.

- **Behaviour**
  - Only the **leader** writes heartbeats:
    - Periodic `UPSERT` of its `(provider_id, instance_id, now())` row.
  - Followers and external tooling can:
    - Read `leader_heartbeat` to see which instance is currently leader and when it last refreshed.
  - Election still relies **only** on advisory locks, so there is no split-brain risk from stale heartbeat data.

Implementation of this table can be added in a later migration; the first version of the feature can work without it if desired.

---

### 5. BlockchainService wiring

We extend `client/blockchain-service` to be role-aware and to use the new DB helpers.

#### 5.1 New fields

In `BlockchainService<FSH, Runtime>`:

- Add:

  - `pub(crate) role: NodeRole`:
    - `enum NodeRole { Leader, Follower }`.
  - `pub(crate) leadership_conn: Option<tokio_postgres::Client>`:
    - Handle to the dedicated, TLS-enabled leadership connection used for advisory locks.

These fields will be initialised in `BlockchainService::new` / event loop startup and used to gate behaviour.

#### 5.2 Startup sequence

In `BlockchainServiceEventLoop::run`:

1. **Initialise pending-tx DB and leadership (inside `init_pending_tx_store`)**:

   - `init_pending_tx_store` becomes responsible for:
     - Reading the `pending_db_url` (config or `SH_PENDING_DB_URL`).
     - If URL is present:
       - Creating the async `DbPool` via `setup_db_pool` and storing `pending_tx_store: Some(PendingTxStore)` on success.
       - Creating a TLS-capable leadership connection via `open_leadership_connection`.
       - Calling `try_acquire_leadership(LEADERSHIP_LOCK_KEY)` on that connection to determine the role:
         - If `true` → set `role = Leader` and keep the leadership connection on the actor.
         - If `false` → set `role = Follower` and keep the leadership connection on the actor.
     - If URL is **absent** or pool creation fails:
       - Log at `info`/`warn` and set `role = Standalone`.
       - Leave `pending_tx_store` and `leadership_conn` as `None`.

2. **Role-specific initialisation after `init_pending_tx_store`**:

   - After `init_pending_tx_store().await`, `BlockchainServiceEventLoop::run` matches on `actor.role`:
     - If `role == Leader`:
       - Call `resubscribe_pending_transactions_on_startup` as today.
         - This reconstructs watchers and transaction manager state from the DB.
       - (Later) start any leader-only background tasks (e.g. heartbeat writer).
     - If `role == Follower`:
       - Call a new, documented helper (e.g. `init_follower_pending_tx_state`) that will eventually:
         - Perform the follower startup snapshot from DB (see section 6).
         - Start a LISTEN loop for `pending_tx_new` / `pending_tx_update`.
         - Start a periodic polling task to repair missed notifications.
       - For the initial implementation, this helper will be a stub with a `TODO` comment and minimal logging.
     - If `role == Standalone`:
       - Do not perform any DB-backed pending-tx initialisation.
       - Log that the node is running in standalone mode with no shared pending-tx DB.

3. **Promotion attempts (followers only)**:

   - Followers run a periodic task (e.g. every few seconds with jitter) that:
     - Calls `try_acquire_leadership` on the stored leadership connection.
     - If it returns `true`:
       - Promote to `role = Leader`.
       - Start leader-only tasks:
         - `resubscribe_pending_transactions_on_startup`.
         - Heartbeat writer.
       - Optionally stop follower-only DB→TransactionManager tasks or convert them into leader mode.

Followers **must not** promote based on any condition other than successfully acquiring the advisory lock.

---

### 6. Follower: DB → TransactionManager bridge (Option B)

Followers will maintain their own `TransactionManager`, driven entirely by DB state and notifications, with richer information (Option B).

#### 6.1 Mapping DB state to TransactionStatus

- Extend `PendingTxStore::update_state` (and possibly schema) so that, when leader watchers see a `TransactionStatus<Hash, Hash>`:

  - They can provide:
    - The DB `state` string (already mapped via `status_to_db_state`).
    - Optionally:
      - `block_hash` (for `InBlock`, `Finalized`, `FinalityTimeout`, `Retracted`, etc.).
      - Any extra data needed for better diagnostics.
  - The DB layer:
    - Persists what is necessary (for audit and follower correctness).
    - Includes these details in NOTIFY payloads.

- Followers receive notifications including:
  - `(account_id, nonce, hash, state, maybe_block_hash, updated_at, ...)`.
  - They reconstruct an appropriate `TransactionStatus` value:
    - For example:
      - `"in_block"` + `block_hash` → `TransactionStatus::InBlock((block_hash, _))`.
      - `"finalized"` + `block_hash` → `TransactionStatus::Finalized((block_hash, _))`.
      - `"dropped"` → `TransactionStatus::Dropped`.
      - `"invalid"` → `TransactionStatus::Invalid`.
      - etc.
  - Where we do not have the exact extra tuple elements (e.g. full `Header`), we can either:
    - Use placeholder values where the type allows, or
    - Store and reconstruct only what is needed to satisfy `TransactionManager` and any watcher APIs we care about on followers.

#### 6.2 Initial snapshot on follower startup and on promotion to leader

There are two important times when a node should initialise its `TransactionManager` from the DB:

1. **Started as Leader**:

   - When a node starts up in `Leader` role, it should initialise its `TransactionManager` from the DB so that:
     - It knows which non-terminal pending transactions exist for its account.
     - It can take over tracking, gap detection, and cleanup correctly.

2. **Promotion to Leader**:
   - When a node transitions from `Follower` to `Leader`, it **must** initialise its `TransactionManager` from the DB so that:
     - It knows which non-terminal pending transactions exist for its account.
     - It can take over tracking, gap detection, and cleanup correctly.

The initialisation process in both cases is the same:

1. Resolve the node’s signing `AccountId` (same as leader uses).
2. Query DB for all rows in `pending_transactions` for this account where `state` is non-terminal.
3. For each row:
   - Decode `call_scale` into `Runtime::Call` if present and non-empty; otherwise `None`.
   - Create a `PendingTransaction` and insert it into `TransactionManager` via `track_transaction`:
     - `nonce` from row.
     - `hash` from row.
     - `call` from decoded `call_scale`.
     - `tip` can be set to `0` or some default (we likely do not persist it now).
     - `submitted_at` can be approximated:
       - Either from `created_at` mapped to a block number heuristic, or
       - Simply use the current best block number.
   - Set `latest_status` based on the DB `state` + any stored block hash information.
4. This seeds the `TransactionManager` so that any existing in-progress transactions are visible to higher-level code and can be managed by a newly promoted leader.

#### 6.3 Leader NOTIFY-driven updates (future work)

In the revised design, **followers do not track extrinsics via Postgres**. They do not maintain a live `TransactionManager` view driven by DB notifications; instead, they only initialise their `TransactionManager` when they are (or become) leaders (see section 6.2).

However, it may still be useful in the future for the **leader** to use LISTEN/NOTIFY or periodic polling for:

- Cross-process observability of pending transactions.
- Potential external tooling that wants to watch pending-tx state changes via DB notifications.

Those concerns are orthogonal to the internal `TransactionManager` used by the leader and can be designed later if needed.

For now:

- Only leaders track extrinsics via their in-memory `TransactionManager`.
- Followers do **not** run any DB→`TransactionManager` bridge; they simply initialise from DB on startup (if they start as leader) or on promotion to leader.

---

### 7. Leader-only vs follower-only paths

Once `role` is available, we clearly divide behaviour:

- **Leader-only**

  - `send_extrinsic`:
    - Only invoked in `Leader` or `Standalone` role.
    - In follower mode, this path should not be used; if it is accidentally called, it should:
      - Return a clear error and log a warning.
  - `send_gap_filling_transaction`:
    - Only invoked in `Leader` or `Standalone` role.
  - `cleanup_tx_manager_and_handle_nonce_gaps`:
    - Only leader/standalone nodes run nonce gap detection and gap-filling.
  - `cleanup_pending_tx_store`:
    - Only leader/standalone nodes perform DB cleanup of old pending transactions.
  - Emitting internal events that trigger tasks:
    - Capacity management, proofs submission, storage request handling, etc., are only driven by the leader.
  - Any future write paths that change on-chain state.

- **Shared (leader + follower)**
  - Read-only runtime queries.
  - Forest root processing and related logic, so that followers keep their Forests in sync.
  - Block/finality notification handling for read-side effects that are safe on all nodes.
  - Event bus emissions for read-only or user-facing notifications (as appropriate).

This ensures that **only the leader interacts with nonce/tx-pool semantics and task orchestration**, while followers keep their local state and data in sync and are ready to assume leadership when promoted.

---

### 8. Behaviour when DB is unhealthy (followers)

On followers:

- Any sustained failure in the pending-tx DB layer (LISTEN connection errors, repeated query failures) must be logged **very loudly**, including:
  - That the node is a **follower**.
  - That its transaction view is now **stale / unreliable**.
- Followers **must not** promote to leader unless they can:
  - Maintain a healthy leadership connection, and
  - Successfully acquire the advisory lock.
- Optional degradation:
  - Followers may continue to serve purely read-only blockchain queries.
  - Any APIs that depend on accurate transaction status (particularly those that wait for `InBlock`/`Finalized`) should be clearly documented as:
    - Reliable only on the leader, or
    - Best-effort on followers while DB is degraded.

---

### 9. Implementation phases (suggested order)

1. **Leadership infrastructure**

   - Implement TLS-capable leadership connection helpers in the DB crate.
   - Add `NodeRole` and `leadership_conn` to `BlockchainService`.
   - Implement advisory-lock-based leader selection at startup.
   - Guard `send_extrinsic` / `send_gap_filling_transaction` so only the leader can call them.

2. **DB notifications for updates**

   - Add NOTIFY support on updates (trigger or explicit calls).
   - Define and implement the JSON payload shape for inserts and updates (including Option B fields).

3. **Follower snapshot + NOTIFY integration**

   - Implement follower startup snapshot seeding `TransactionManager`.
   - Implement LISTEN-based update handling and follower-side status update logic.
   - Add periodic poll/repair task.

4. **Heartbeat (optional observability)**

   - Add `leader_heartbeat` table and leader heartbeat writer task.
   - Integrate with monitoring / logging where appropriate.

5. **Refinement and tests**
   - Add integration tests (e.g. in `test/suites/integration/msp/multi-msp-instances.test.ts`) to:
     - Start two or more nodes against the same DB/keystore.
     - Assert one becomes leader, others followers.
     - Submit transactions via leader and verify that followers see consistent statuses via their `TransactionManager`.
   - Tune logging to make role and DB health very clear at runtime.
