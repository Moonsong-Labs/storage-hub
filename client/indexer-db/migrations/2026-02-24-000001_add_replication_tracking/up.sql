ALTER TABLE file ADD COLUMN bsps_required INTEGER NOT NULL DEFAULT 0;
ALTER TABLE file ADD COLUMN desired_replicas INTEGER NOT NULL DEFAULT 0;

-- Backfill desired_replicas for existing files to match their current BSP count,
-- so the UI does not flag them as under-replicated.
UPDATE file SET desired_replicas = (
    SELECT COUNT(*) FROM bsp_file WHERE bsp_file.file_id = file.id
);

ALTER TABLE bsp_file DROP CONSTRAINT bsp_file_bsp_id_fkey;
ALTER TABLE bsp_file ADD CONSTRAINT bsp_file_bsp_id_fkey
    FOREIGN KEY (bsp_id) REFERENCES bsp(id) ON DELETE CASCADE;
