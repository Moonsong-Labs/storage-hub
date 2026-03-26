-- Remove the legacy DB bucket reference from files.
-- file.onchain_bucket_id is now the canonical bucket identifier.
DROP INDEX IF EXISTS idx_file_bucket_id;

ALTER TABLE file DROP COLUMN bucket_id;
