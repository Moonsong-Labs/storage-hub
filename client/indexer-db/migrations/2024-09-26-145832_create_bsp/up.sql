-- Create BSP table
CREATE TABLE bsp (
    id SERIAL PRIMARY KEY,
    account VARCHAR NOT NULL,
    capacity NUMERIC(20, 0) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create BSP_MultiAddress table
CREATE TABLE bsp_multiaddress (
    bsp_id INTEGER NOT NULL,
    multiaddress_id INTEGER NOT NULL,
    PRIMARY KEY (bsp_id, multiaddress_id),
    FOREIGN KEY (bsp_id) REFERENCES bsp(id) ON DELETE CASCADE,
    FOREIGN KEY (multiaddress_id) REFERENCES multiaddress(id) ON DELETE CASCADE
);

-- Create index on bsp_id for faster lookups
CREATE INDEX idx_bsp_multiaddress_bsp_id ON bsp_multiaddress(bsp_id);
