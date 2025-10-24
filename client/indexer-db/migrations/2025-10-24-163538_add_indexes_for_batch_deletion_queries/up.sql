-- Indexes to optimize fisherman batch deletion queries
-- Support get_files_pending_deletion_grouped_by_bsp and get_files_pending_deletion_grouped_by_bucket

-- Filter BSPs by onchain ID
CREATE INDEX idx_bsp_onchain_bsp_id ON bsp(onchain_bsp_id);

-- Filter and order files by bucket ID
CREATE INDEX idx_file_onchain_bucket_id ON file(onchain_bucket_id);

-- Composite index for efficient bucket-grouped deletion queries with ordering
CREATE INDEX idx_file_deletion_bucket_order
ON file(onchain_bucket_id, file_key)
WHERE deletion_status IS NOT NULL;
