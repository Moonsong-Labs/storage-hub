-- Add deletion_signature column to file table
-- This stores the user's signature from FileDeletionRequested events
-- Required by fisherman nodes to construct valid proofs for delete_file extrinsic
ALTER TABLE file ADD COLUMN deletion_signature BYTEA DEFAULT NULL;
