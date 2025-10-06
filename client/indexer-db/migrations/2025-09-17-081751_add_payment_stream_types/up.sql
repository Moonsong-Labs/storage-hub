-- Add new columns for fixed and dynamic payment streams
ALTER TABLE paymentstream
ADD COLUMN rate NUMERIC(38, 0) NULL,
ADD COLUMN amount_provided NUMERIC(38, 0) NULL;

-- Set default values for existing rows
-- All will be converted to fixed streams with rate of 1
UPDATE paymentstream
SET rate = 1
WHERE rate IS NULL AND amount_provided IS NULL;

-- Add CHECK constraint to ensure exactly one type is set
ALTER TABLE paymentstream
ADD CONSTRAINT check_payment_stream_type
CHECK (
    (rate IS NOT NULL AND amount_provided IS NULL)
    OR
    (rate IS NULL AND amount_provided IS NOT NULL)
);

-- Create index for rate column for query performance
CREATE INDEX idx_paymentstream_rate ON paymentstream(rate) WHERE rate IS NOT NULL;

-- Create index for amount_provided column for query performance  
CREATE INDEX idx_paymentstream_amount_provided ON paymentstream(amount_provided) WHERE amount_provided IS NOT NULL;
