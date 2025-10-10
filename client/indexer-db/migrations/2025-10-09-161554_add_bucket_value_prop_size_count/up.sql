-- Add value_prop_id, total_size, and file_count columns to bucket table
ALTER TABLE bucket
ADD COLUMN value_prop_id VARCHAR,
ADD COLUMN total_size NUMERIC NOT NULL DEFAULT 0,
ADD COLUMN file_count BIGINT NOT NULL DEFAULT 0;
