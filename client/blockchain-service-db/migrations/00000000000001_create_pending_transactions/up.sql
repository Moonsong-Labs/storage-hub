-- Create the pending_transactions table to coordinate txs across instances
CREATE TABLE IF NOT EXISTS pending_transactions (
  account_id BYTEA NOT NULL,
  nonce BIGINT NOT NULL,
  hash BYTEA NOT NULL,
  call_scale BYTEA NOT NULL,
  -- Full signed extrinsic bytes for re-subscription on restart
  extrinsic_scale BYTEA NOT NULL,
  state TEXT NOT NULL CHECK (state IN (
    'future','ready','broadcast','queued','sent','in_block','retracted','finalized','invalid','dropped','usurped','finality_timeout'
  )),
  creator_id TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, nonce)
);

CREATE INDEX IF NOT EXISTS pending_tx_hash_idx ON pending_transactions (hash);

-- Notification trigger for new pending transactions (for follower instances)
CREATE OR REPLACE FUNCTION notify_pending_tx_new() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify(
    'pending_tx_new',
    json_build_object(
      'account_id', encode(NEW.account_id, 'hex'),
      'nonce', NEW.nonce,
      'hash', encode(NEW.hash, 'hex'),
      'state', NEW.state,
      'creator_id', NEW.creator_id
    )::text
  );
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$ BEGIN
  CREATE TRIGGER trg_pending_tx_new
  AFTER INSERT ON pending_transactions
  FOR EACH ROW EXECUTE FUNCTION notify_pending_tx_new();
EXCEPTION WHEN duplicate_object THEN
  -- Trigger already exists; ignore
  NULL;
END $$;

-- Update timestamp trigger
CREATE OR REPLACE FUNCTION set_updated_at_timestamp() RETURNS trigger AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$ BEGIN
  CREATE TRIGGER trg_pending_tx_updated_at
  BEFORE UPDATE ON pending_transactions
  FOR EACH ROW EXECUTE FUNCTION set_updated_at_timestamp();
EXCEPTION WHEN duplicate_object THEN
  NULL;
END $$;


