-- Restore the legacy DB bucket reference on files.
-- Rebuild values from the canonical on-chain bucket ID.
ALTER TABLE file ADD COLUMN bucket_id BIGINT;

UPDATE file
SET bucket_id = bucket.id
FROM bucket
WHERE file.onchain_bucket_id = bucket.onchain_bucket_id;

ALTER TABLE file
ALTER COLUMN bucket_id SET NOT NULL;

CREATE INDEX idx_file_bucket_id ON file(bucket_id);
