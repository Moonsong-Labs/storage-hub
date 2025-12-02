-- Add tx_hash and block_hash columns to file table
-- 
-- tx_hash: Tracks the transaction hash that created the file:
--   - For EVM transactions: Contains the Ethereum transaction hash (32 bytes)
--   - For Substrate extrinsics: NULL (may be extended in the future to store extrinsic hash)
--   - Defaults to NULL for all existing files since we don't have historical transaction data
-- 
-- block_hash: Tracks the block hash where the file was created:
--   - Contains the block hash (32 bytes) where the NewStorageRequest event was emitted
--   - For existing files, we use a placeholder hash (all zeros) since we don't have historical data
ALTER TABLE file ADD COLUMN block_hash BYTEA NOT NULL DEFAULT '\x0000000000000000000000000000000000000000000000000000000000000000';
ALTER TABLE file ADD COLUMN tx_hash BYTEA DEFAULT NULL;

-- Note: Existing files get a placeholder block_hash (all zeros) since we don't have historical data
-- Only new files created after this migration will have the actual block_hash populated by the indexer
