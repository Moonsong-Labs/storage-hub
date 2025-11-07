-- Drop value_prop_id, total_size, and file_count columns from bucket table
ALTER TABLE bucket
DROP COLUMN value_prop_id,
DROP COLUMN total_size,
DROP COLUMN file_count;
