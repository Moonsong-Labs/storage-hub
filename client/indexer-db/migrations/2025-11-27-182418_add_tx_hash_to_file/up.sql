-- Add tx_hash column to file table
-- This tracks the transaction hash that created the file:
--   - For EVM transactions: Contains the Ethereum transaction hash (32 bytes)
--   - For Substrate extrinsics: NULL (may be extended in the future to store extrinsic hash)
-- Defaults to NULL for all existing files since we don't have historical transaction data
ALTER TABLE file ADD COLUMN tx_hash BYTEA DEFAULT NULL;

-- Note: No UPDATE needed as all existing files should have NULL tx_hash
-- Only new files created after this migration will have tx_hash populated by the indexer
