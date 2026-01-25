-- Drop the leader_info table and related objects
DROP TRIGGER IF EXISTS trg_leader_info_updated_at ON leader_info;
DROP FUNCTION IF EXISTS set_leader_info_updated_at();
DROP TABLE IF EXISTS leader_info;
