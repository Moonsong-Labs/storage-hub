-- Create bsp_file association table
CREATE TABLE bsp_file (
    bsp_id BIGINT NOT NULL,
    file_id BIGINT NOT NULL,
    PRIMARY KEY (bsp_id, file_id),
    FOREIGN KEY (bsp_id) REFERENCES bsp(id),
    FOREIGN KEY (file_id) REFERENCES file(id)
);

-- Create an index on the bsp_id for faster lookups
CREATE INDEX idx_bsp_file_bsp_id ON bsp_file(bsp_id);

-- Create an index on the file_id for faster lookups
CREATE INDEX idx_bsp_file_file_id ON bsp_file(file_id);