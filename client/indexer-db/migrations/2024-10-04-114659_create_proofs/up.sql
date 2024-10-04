CREATE TABLE proofs (
    id SERIAL PRIMARY KEY,
    provider_id VARCHAR NOT NULL,
    last_tick_proof BIGINT NOT NULL DEFAULT 0,
);

-- Create an index on the provider_id column for faster lookups
CREATE INDEX idx_proof_provider_id ON proofs(provider_id);
