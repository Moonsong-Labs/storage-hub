-- Revert bucket on-chain ID uniqueness back to a non-unique index.
DROP INDEX IF EXISTS idx_bucket_blockchain_id;

CREATE INDEX idx_bucket_blockchain_id ON bucket(onchain_bucket_id);
