DROP TRIGGER IF EXISTS trg_pending_tx_updated_at ON pending_transactions;
DROP FUNCTION IF EXISTS set_updated_at_timestamp();

DROP TRIGGER IF EXISTS trg_pending_tx_new ON pending_transactions;
DROP FUNCTION IF EXISTS notify_pending_tx_new();

DROP INDEX IF EXISTS pending_tx_hash_idx;
DROP TABLE IF EXISTS pending_transactions;


