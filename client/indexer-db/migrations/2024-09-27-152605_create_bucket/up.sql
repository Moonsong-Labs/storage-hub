-- Create Bucket table
CREATE TABLE bucket (
    id BIGSERIAL PRIMARY KEY,
    msp_id BIGINT,
    account VARCHAR NOT NULL,
    onchain_bucket_id BYTEA NOT NULL,
    name BYTEA NOT NULL,
    collection_id VARCHAR,
    private BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (msp_id) REFERENCES msp(id) ON DELETE CASCADE
);

-- Create index on msp_id for faster lookups
CREATE INDEX idx_bucket_msp_id ON bucket(msp_id);

-- Create index on account for faster lookups
CREATE INDEX idx_bucket_account ON bucket(account);

-- Create index on blockchain_id for faster lookups
CREATE INDEX idx_bucket_blockchain_id ON bucket(onchain_bucket_id);
