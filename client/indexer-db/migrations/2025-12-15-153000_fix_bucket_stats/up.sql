-- Recalculate bucket file_count and total_size based on files that are linked to the bucket
-- We create a file record every time a new storage request is made,
-- So we can have multiple files with the same file_key (although the bucket only has one file per file_key)
-- We also filter by is_in_bucket = true because files may not yet be accepted into the bucket.

UPDATE bucket
SET
    file_count = (
        SELECT COUNT(*)
        FROM (
            SELECT DISTINCT file_key
            FROM file
            WHERE
                file.onchain_bucket_id = bucket.onchain_bucket_id
                AND file.is_in_bucket = true
        ) f
    ),
   total_size = (
        SELECT COALESCE(SUM(size), 0)
        FROM (
            SELECT DISTINCT ON (file_key) file_key, size
            FROM file
            WHERE
                file.onchain_bucket_id = bucket.onchain_bucket_id
                AND file.is_in_bucket = true
            ORDER BY file_key, created_at DESC
        ) f
    );
