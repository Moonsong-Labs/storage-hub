-- Create MSP table
CREATE TABLE msp (
    id SERIAL PRIMARY KEY,
    account VARCHAR NOT NULL,
    capacity NUMERIC(20, 0) NOT NULL,
    value_prop VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create MSP_MultiAddress table
CREATE TABLE msp_multiaddress (
    msp_id INTEGER NOT NULL,
    multiaddress_id INTEGER NOT NULL,
    PRIMARY KEY (msp_id, multiaddress_id),
    FOREIGN KEY (msp_id) REFERENCES msp(id) ON DELETE CASCADE,
    FOREIGN KEY (multiaddress_id) REFERENCES multiaddress(id) ON DELETE CASCADE
);

-- Create index on msp_id for faster lookups
CREATE INDEX idx_msp_multiaddress_msp_id ON msp_multiaddress(msp_id);
