-- Enforce bucket on-chain ID uniqueness.
-- This matches current code assumptions that bucket lookups by onchain_bucket_id
-- resolve to a single canonical bucket row.
DROP INDEX IF EXISTS idx_bucket_blockchain_id;

CREATE UNIQUE INDEX idx_bucket_blockchain_id ON bucket(onchain_bucket_id);
