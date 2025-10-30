## Postgres-backed Leader Election and Acting Gate for MSP

### Overview

Enable multiple MSP instances to run concurrently while ensuring that only one instance (the leader) sends extrinsics. Followers stay hot and react-only. Leadership is coordinated via Postgres advisory locks; a small table stores epoch/heartbeat for fencing and observability; a local acting gate blocks extrinsic-sending unless this instance is the current leader.

### Objectives

- Ensure only one MSP instance acts (sends extrinsics) at any time.
- Fast, safe failover when the leader dies or loses connectivity.
- Infra-agnostic: reuse existing Postgres; no new distributed system required.
- Minimal, testable module with clear integration points.

### Non-goals

- Cross-datacenter consensus beyond the guarantees of Postgres.
- Per-operation distributed transactions. The acting gate is the single choke point.

---

## Architecture

- Leader selection: Postgres advisory locks (`pg_try_advisory_lock`) provide single-winner mutual exclusion. The lock is tied to a dedicated session; it’s released on session death.
- Fencing and observability: a single-row table `msp_leader_state` tracks `epoch`, `holder_identity`, and `last_heartbeat`.
- Acting gate: in-process guard that only allows extrinsic-sending if this instance holds leadership for the current `epoch`.
- Optional: `LISTEN/NOTIFY` to nudge followers on leadership changes (faster wake-ups).

---

## Components

### Leader identity and configuration

- Instance identity: `<network>-<msp_account>-<hostname>-<pid>-<uuid4>`.
- Scope and lock key: derive a 64-bit key from a stable scope string, e.g. `hash64("storagehub:<network>:<msp_account>:msp-leader")`.
- Config knobs (env/config):
  - `LEADER_LOCK_KEY` (optional override)
  - `PG_CONN_STRING` (dedicated connection for lock/heartbeat)
  - `HEARTBEAT_INTERVAL_MS` (default 3000–5000)
  - `HEARTBEAT_LATE_THRESHOLD_MS` (~2x interval)
  - `ACQUIRE_BACKOFF_MS_MIN`/`MAX` (e.g., 200/800; jittered)
  - `EPOCH_STALE_THRESHOLD_MS` (optional, for follower-side monitoring)
  - `PG_LISTEN_CHANNEL` (optional, e.g., `msp_leader_change`)

### Database schema

Purpose: observability + fencing token storage.

```sql
CREATE TABLE IF NOT EXISTS msp_leader_state (
  id                SMALLINT PRIMARY KEY DEFAULT 1,
  scope_key         BIGINT NOT NULL,
  epoch             BIGINT NOT NULL,
  holder_identity   TEXT   NOT NULL,
  last_heartbeat    TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO msp_leader_state (id, scope_key, epoch, holder_identity, last_heartbeat)
VALUES (1, :scope_key, 0, '', now())
ON CONFLICT (id) DO NOTHING;
```

Optional notify trigger:

```sql
CREATE OR REPLACE FUNCTION msp_leader_notify() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('msp_leader_change', NEW.holder_identity || ':' || NEW.epoch);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS msp_leader_state_notify ON msp_leader_state;
CREATE TRIGGER msp_leader_state_notify
AFTER UPDATE ON msp_leader_state
FOR EACH ROW EXECUTE FUNCTION msp_leader_notify();
```

### Advisory lock semantics

- Use a dedicated, long‑lived DB session to hold the advisory lock (do not pool/rotate).
- Acquire loop:
  - `SELECT pg_try_advisory_lock(:scope_key)` → on success, you’re leader; on failure, follower.
- Release on shutdown with `pg_advisory_unlock(:scope_key)`; on crash, Postgres releases the lock automatically.

### Epoch and heartbeat (lease-like behavior)

- On becoming leader:

```sql
UPDATE msp_leader_state
SET epoch = epoch + 1, holder_identity = :me, last_heartbeat = now()
WHERE id = 1 AND scope_key = :scope_key
RETURNING epoch;
```

- Heartbeat loop (every `HEARTBEAT_INTERVAL_MS`, with jitter):

```sql
UPDATE msp_leader_state SET last_heartbeat = now() WHERE id = 1;
```

- If heartbeat update fails or drifts beyond `HEARTBEAT_LATE_THRESHOLD_MS`, disable the acting gate immediately and re-enter acquire flow.

### Fencing token and acting gate

- Fencing token: the `epoch` returned upon leadership acquisition.
- `LeaderGate` maintains:
  - `is_leader` (atomic), `current_epoch` (atomic), `holder_identity`.
- All extrinsic-sending paths must call `require_leader()` and re-validate epoch just before send.
- On leadership loss or epoch change, the gate disables and rejects new sends; in-flight must re-check before transmission.

---

## Integration in this repository

- Place the election module in `client/common` or a new `client/leader-election` crate used by `client/blockchain-service` and task modules.
- Gate placement: wrap the single choke point that signs/sends extrinsics in `client/blockchain-service` (e.g., in `handler.rs`) with `LeaderGate`.
- Ensure all direct extrinsic submissions elsewhere route through this gate (refactor if needed).
- Broadcast `LeaderStateChanged { is_leader, epoch }` via the actors framework so other services can pause/resume safely.

---

## Proposed APIs (sketch)

```rust
// Identity and token used by the acting gate
pub struct LeaderToken {
    pub epoch: i64,
    pub holder_identity: String,
}

pub struct LeaderGate { /* atomics + state */ }

impl LeaderGate {
    pub fn is_leader(&self) -> bool { /* ... */ }
    pub async fn require_leader(&self) -> Result<LeaderToken, NotLeader> { /* ... */ }
    pub fn enable(&self, token: LeaderToken) { /* ... */ }
    pub fn disable(&self) { /* ... */ }
    pub fn ensure_epoch(&self, epoch: i64) -> Result<(), NotLeader> { /* just-in-time check */ }
}

pub struct LeaderElectionService { /* holds dedicated PG connection */ }

impl LeaderElectionService {
    pub async fn start(scope_key: i64, identity: String) -> (LeaderGate, JoinHandle<()>) { /* ... */ }
}
```

Acquire loop (pseudocode):

```rust
loop {
    if pg_try_advisory_lock(scope_key).await? {
        let epoch = bump_epoch_and_heartbeat(identity.clone()).await?;
        gate.enable(LeaderToken { epoch, holder_identity: identity.clone() });
        // heartbeat returns on failure or shutdown
        run_heartbeat_loop(&gate).await;
        gate.disable();
        let _ = pg_advisory_unlock(scope_key).await; // best-effort
    } else {
        gate.disable();
        sleep_with_jitter(backoff_min, backoff_max).await;
        // optionally LISTEN for msp_leader_change to wake early
    }
}
```

Heartbeat (pseudocode):

```rust
loop {
    sleep_with_jitter(heartbeat_interval).await;
    // proactive safety
    if now() - last_successful_heartbeat > late_threshold { gate.disable(); }
    update_last_heartbeat().await?;
}
```

Send path guard (pseudocode):

```rust
let token = gate.require_leader().await?;
// build extrinsic...
gate.ensure_epoch(token.epoch)?; // just-in-time recheck
// send
```

---

## Failure modes and mitigations

- Stuck leader with live DB session: watchdog disables gate if heartbeats are late; leadership remains held, preserving safety. Gate re-opens after heartbeats resume.
- Connection pooling drops lock: use a dedicated connection for the advisory lock and heartbeat; never return it to the pool while leader.
- Clock skew: use DB `now()` for heartbeat writes; followers should treat timestamp gaps as observability signals, not safety conditions.
- Scope collisions: include network identifier and MSP account in the lock key derivation.

---

## Backoff, jitter, and timings

- Heartbeat interval: 2–5s; jitter ±20% to avoid lockstep.
- Late threshold: ~2× heartbeat interval (tune to your tolerance).
- Acquire backoff for followers: 200–800ms jitter; exponential backoff for transient DB errors, capped to a few seconds.

---

## Observability

- Metrics:
  - Counters: `leader_acquired_total`, `leadership_lost_total`, `acquire_attempts_total`, `extrinsic_blocked_not_leader_total`.
  - Gauges: `is_leader` (0/1), `leader_epoch`, `heartbeat_age_ms`.
  - Histograms: `acquire_latency_ms`, `heartbeat_update_ms`.
- Logs: include `scope_key`, `holder_identity`, `epoch`, reason on transitions.
- Optional: expose role and epoch via health/metrics endpoint.

---

## Security and permissions

- DB user requires:
  - Access to `pg_try_advisory_lock`/`pg_advisory_unlock`.
  - `SELECT/UPDATE` on `msp_leader_state`.
  - `LISTEN/UNLISTEN` if NOTIFY is used.
- Prefer a separate credentials/connection for the leadership session.

---

## Configuration and rollout

- Feature flag: `MSP_LEADER_ENABLED=true|false`.
- If unset, choose an explicit default (recommended: disabled → follower) or maintain current single-instance behavior (always leader) for backward compatibility.
- Canary two instances against a shared Postgres in a non-prod environment; tune intervals.

---

## Testing

- Unit tests:
  - Lock acquisition logic (mock DB).
  - Gate state transitions and fencing token checks.
- Integration (Docker):
  - Two MSP processes + one Postgres.
  - Verify only one sends extrinsics.
  - Kill leader; follower takes over within thresholds.
  - Restart DB; ensure re-acquire works; no split-brain.
  - Introduce CPU pauses; watchdog disables gate on late heartbeat.

---

## Acceptance criteria

- Only one instance sends extrinsics at any moment.
- Failover occurs within configured heartbeat thresholds.
- No split-brain under tested failure modes.
- Metrics/logs expose role, epoch, and transitions.
- All extrinsic paths are gated through `LeaderGate`.

---

## Operational notes

- Keep Postgres HA and monitoring in place.
- Alert on `heartbeat_age_ms` beyond threshold or frequent leadership flaps.
- Document a “leader drain” procedure: disable gate, unlock, then stop.
