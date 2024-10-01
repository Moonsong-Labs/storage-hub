CREATE TABLE paymentstream (
    id SERIAL PRIMARY KEY,
    account VARCHAR NOT NULL,
    provider VARCHAR NOT NULL,
    total_amount NUMERIC DEFAULT 0,
);

-- Create an index on the account column for faster lookups
CREATE INDEX idx_paymentstream_account ON paymentstream(account);

-- Create an index on the provider column for faster lookups
CREATE INDEX idx_paymentstream_provider ON paymentstream(provider);
