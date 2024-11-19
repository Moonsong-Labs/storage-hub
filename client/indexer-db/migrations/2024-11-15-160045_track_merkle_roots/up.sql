-- Add merkle_root columns to bucket and bsp tables
ALTER TABLE bucket ADD COLUMN merkle_root BYTEA NOT NULL;
ALTER TABLE bsp ADD COLUMN merkle_root BYTEA NOT NULL;
