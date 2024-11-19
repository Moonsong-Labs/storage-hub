-- Remove merkle_root columns from bucket and bsp tables
ALTER TABLE bucket DROP COLUMN merkle_root;
ALTER TABLE bsp DROP COLUMN merkle_root;
