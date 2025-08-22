-- Create msp_file association table
CREATE TABLE msp_file (
    msp_id BIGINT NOT NULL,
    file_id BIGINT NOT NULL,
    PRIMARY KEY (msp_id, file_id),
    FOREIGN KEY (msp_id) REFERENCES msp(id),
    FOREIGN KEY (file_id) REFERENCES file(id)
);

-- Create an index on the msp_id for faster lookups
CREATE INDEX idx_msp_file_msp_id ON msp_file(msp_id);

-- Create an index on the file_id for faster lookups
CREATE INDEX idx_msp_file_file_id ON msp_file(file_id);