CREATE TABLE file (
    id BIGSERIAL PRIMARY KEY,
    account BYTEA NOT NULL,
    file_key BYTEA NOT NULL,
    bucket_id INTEGER NOT NULL,
    location BYTEA NOT NULL,
    fingerprint BYTEA NOT NULL,
    size BIGINT NOT NULL,
    step INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create and index on file_key for faster lookups
CREATE INDEX idx_file_file_key ON file(file_key);

-- Create an index on the bucket_id for faster lookups
CREATE INDEX idx_file_bucket_id ON file(bucket_id);

-- Create Bsp_File table
CREATE TABLE bsp_file (
    bsp_id INTEGER NOT NULL,
    file_id BIGINT NOT NULL,
    PRIMARY KEY (bsp_id, file_id),
    FOREIGN KEY (file_id) REFERENCES file(id) ON DELETE CASCADE
);

-- Create an index on the bsp_id for faster lookups
CREATE INDEX idx_bsp_file_bsp_id ON bsp_file(bsp_id);

-- Create an index on the file_id for faster lookups
CREATE INDEX idx_bsp_file_file_id ON bsp_file(file_id);
