-- Remove tx_hash and block_hash columns from file table
-- WARNING: This will permanently delete all transaction and block hash data
ALTER TABLE file DROP COLUMN block_hash;
ALTER TABLE file DROP COLUMN tx_hash;

