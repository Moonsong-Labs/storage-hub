CREATE TABLE multiaddress (
    id BIGSERIAL PRIMARY KEY,
    address BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create an index on the address column for faster lookups
CREATE INDEX idx_multiaddress_address ON multiaddress(address);
