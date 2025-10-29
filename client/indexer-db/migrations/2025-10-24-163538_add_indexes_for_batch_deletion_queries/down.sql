-- Remove batch deletion query indexes and schema changes
DROP INDEX IF EXISTS idx_file_deletion_bucket_order;
DROP INDEX IF EXISTS idx_file_deletion_requested_at;
DROP INDEX IF EXISTS idx_file_onchain_bucket_id;
DROP INDEX IF EXISTS idx_bsp_onchain_bsp_id;
ALTER TABLE file DROP COLUMN deletion_requested_at;
