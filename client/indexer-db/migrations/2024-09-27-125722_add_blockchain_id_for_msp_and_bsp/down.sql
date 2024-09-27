-- Remove blockchain_id column from bsp table
ALTER TABLE bsp DROP COLUMN blockchain_id;

-- Remove blockchain_id column from msp table
ALTER TABLE msp DROP COLUMN blockchain_id;
