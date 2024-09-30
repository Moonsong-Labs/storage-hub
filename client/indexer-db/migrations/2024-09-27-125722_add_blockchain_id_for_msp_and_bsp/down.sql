-- Remove onchain_bsp_id column from bsp table
ALTER TABLE bsp DROP COLUMN onchain_bsp_id;

-- Remove onchain_msp_id column from msp table
ALTER TABLE msp DROP COLUMN onchain_msp_id;
