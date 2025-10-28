-- Indexes and schema changes to optimize fisherman batch deletion queries
-- Support get_files_pending_deletion_grouped_by_bsp and get_files_pending_deletion_grouped_by_bucket

-- Add deletion_requested_at timestamp for proper FIFO ordering
ALTER TABLE file ADD COLUMN deletion_requested_at TIMESTAMP DEFAULT NULL;

-- Filter BSPs by onchain ID
CREATE INDEX idx_bsp_onchain_bsp_id ON bsp(onchain_bsp_id);

-- Filter and order files by bucket ID
CREATE INDEX idx_file_onchain_bucket_id ON file(onchain_bucket_id);

-- Index for efficient FIFO ordering of pending deletions
CREATE INDEX idx_file_deletion_requested_at
ON file(deletion_requested_at)
WHERE deletion_status IS NOT NULL;

-- Composite index for efficient bucket-grouped deletion queries with ordering
-- Orders by bucket, FIFO (deletion_requested_at), then file_key for deterministic ordering
CREATE INDEX idx_file_deletion_bucket_order
ON file(onchain_bucket_id, deletion_requested_at, file_key)
WHERE deletion_status IS NOT NULL;
