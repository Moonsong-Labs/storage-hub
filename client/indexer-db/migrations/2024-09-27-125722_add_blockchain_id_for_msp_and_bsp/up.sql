-- Add blockchain_id column to bsp table
ALTER TABLE bsp ADD COLUMN blockchain_id VARCHAR NOT NULL DEFAULT '';

-- Add blockchain_id column to msp table
ALTER TABLE msp ADD COLUMN blockchain_id VARCHAR NOT NULL DEFAULT '';

-- Remove the default value after adding the column
ALTER TABLE bsp ALTER COLUMN blockchain_id DROP DEFAULT;
ALTER TABLE msp ALTER COLUMN blockchain_id DROP DEFAULT;
