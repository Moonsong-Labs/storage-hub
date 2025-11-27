-- Remove tx_hash column from file table
-- WARNING: This will permanently delete all transaction hash data
ALTER TABLE file DROP COLUMN tx_hash;

