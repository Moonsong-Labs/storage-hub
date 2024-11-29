CREATE TABLE peer_id (
    id SERIAL PRIMARY KEY,
    peer BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create File_PeerId table
CREATE TABLE file_peer_id (
    file_id INTEGER NOT NULL,
    peer_id INTEGER NOT NULL,
    PRIMARY KEY (file_id, peer_id),
    FOREIGN KEY (file_id) REFERENCES file(id) ON DELETE CASCADE,
    FOREIGN KEY (peer_id) REFERENCES peer_id(id) ON DELETE CASCADE
);

-- Create index on file_id for faster lookups
CREATE INDEX idx_file_peer_id_file_id ON file_peer_id(file_id);
