-- This migration cannot be safely reversed because we don't track which specific
-- file records had incorrect is_in_bucket=false values before the normalization.
--
-- Reverting this migration would require arbitrarily setting some records back to
-- is_in_bucket=false, which could recreate the inconsistent state we're trying to fix.
--
-- If you need to rollback, the safest approach is to restore from a database backup
-- taken before this migration was applied.

-- NOOP query required to not break the revertion

SELECT 1;
