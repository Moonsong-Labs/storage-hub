-- Add is_in_bucket column to file table
-- This tracks whether a file is currently in the bucket's forest based on MutationsApplied events
-- Defaults to TRUE for existing files, but files without MSP associations are set to FALSE
ALTER TABLE file ADD COLUMN is_in_bucket BOOLEAN NOT NULL DEFAULT TRUE;

-- Update existing files that do NOT have MSP associations to FALSE
-- Files without MSP associations are not yet in the bucket's forest
UPDATE file 
SET is_in_bucket = FALSE 
WHERE id NOT IN (SELECT file_id FROM msp_file);

