-- Create a singleton table to store leader information
-- The leader (node holding the advisory lock) writes its metadata here
-- Metadata is JSON format containing host/port information
CREATE TABLE IF NOT EXISTS leader_info (
  id INTEGER PRIMARY KEY CHECK (id = 1), -- Enforce singleton (only one row allowed)
  metadata JSONB NOT NULL,               -- Leader metadata 
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Pre-insert the singleton row to simplify upserts
INSERT INTO leader_info (id, metadata, updated_at)
VALUES (1, '{}'::jsonb, now())
ON CONFLICT (id) DO NOTHING;

-- Update timestamp trigger for leader_info
CREATE OR REPLACE FUNCTION set_leader_info_updated_at() RETURNS trigger AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$ BEGIN
  CREATE TRIGGER trg_leader_info_updated_at
  BEFORE UPDATE ON leader_info
  FOR EACH ROW EXECUTE FUNCTION set_leader_info_updated_at();
EXCEPTION WHEN duplicate_object THEN
  NULL;
END $$;
