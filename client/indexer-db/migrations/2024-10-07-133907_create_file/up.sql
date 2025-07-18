CREATE TABLE file (
    id BIGSERIAL PRIMARY KEY,
    account BYTEA NOT NULL,
    file_key BYTEA NOT NULL,
    bucket_id BIGINT NOT NULL,
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
