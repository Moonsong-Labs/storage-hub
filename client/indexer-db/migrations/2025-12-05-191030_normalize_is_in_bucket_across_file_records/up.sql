-- Normalize is_in_bucket status across all file records with the same file_key
--
-- If ANY file record with a given file_key has is_in_bucket=true, then ALL
-- records for that file_key should have is_in_bucket=true. This is because
-- the bucket forest only contains one instance of each file_key, so if it's
-- in the bucket, all storage request records for that file should reflect that.

UPDATE file
SET is_in_bucket = true
WHERE file_key IN (
    -- Find all file_keys where at least one record has is_in_bucket=true
    SELECT DISTINCT file_key
    FROM file
    WHERE is_in_bucket = true
)
AND is_in_bucket = false;
