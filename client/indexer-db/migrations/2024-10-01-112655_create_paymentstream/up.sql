CREATE TABLE paymentstream (
    id BIGSERIAL PRIMARY KEY,
    account VARCHAR NOT NULL,
    provider VARCHAR NOT NULL,
    total_amount_paid NUMERIC(38, 0) NOT NULL DEFAULT 0,
    last_tick_charged BIGINT NOT NULL DEFAULT 0,
    charged_at_tick BIGINT NOT NULL DEFAULT 0
);

-- Create an index on the account column for faster lookups
CREATE INDEX idx_paymentstream_account ON paymentstream(account);

-- Create an index on the provider column for faster lookups
CREATE INDEX idx_paymentstream_provider ON paymentstream(provider);
