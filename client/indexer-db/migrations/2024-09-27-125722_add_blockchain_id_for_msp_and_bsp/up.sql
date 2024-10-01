-- Add onchain_bsp_id column to bsp table
ALTER TABLE bsp ADD COLUMN onchain_bsp_id VARCHAR NOT NULL DEFAULT '';

-- Add onchain_msp_id column to msp table
ALTER TABLE msp ADD COLUMN onchain_msp_id VARCHAR NOT NULL DEFAULT '';

-- Remove the default value after adding the column
ALTER TABLE bsp ALTER COLUMN onchain_bsp_id DROP DEFAULT;
ALTER TABLE msp ALTER COLUMN onchain_msp_id DROP DEFAULT;
