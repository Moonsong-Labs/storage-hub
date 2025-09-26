-- Drop indexes first
DROP INDEX IF EXISTS idx_paymentstream_rate;
DROP INDEX IF EXISTS idx_paymentstream_amount_provided;

-- Drop constraint
ALTER TABLE paymentstream DROP CONSTRAINT IF EXISTS check_payment_stream_type;

-- Drop columns
ALTER TABLE paymentstream 
DROP COLUMN IF EXISTS rate,
DROP COLUMN IF EXISTS amount_provided;
