ALTER TABLE file DROP COLUMN bsps_required;
ALTER TABLE file DROP COLUMN desired_replicas;

ALTER TABLE bsp_file DROP CONSTRAINT bsp_file_bsp_id_fkey;
ALTER TABLE bsp_file ADD CONSTRAINT bsp_file_bsp_id_fkey
    FOREIGN KEY (bsp_id) REFERENCES bsp(id);
