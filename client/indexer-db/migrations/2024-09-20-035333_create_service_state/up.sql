CREATE TABLE service_state (
    id INT PRIMARY KEY CHECK (id = 1),                              -- Enforce only one row
    last_indexed_finalized_block BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

INSERT INTO service_state (id, last_indexed_finalized_block) VALUES (1, 0);
