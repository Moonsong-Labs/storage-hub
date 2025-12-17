-- This migration cannot be safely reversed because we don't track the previous
-- file_count and total_size values before the recalculation.
--
-- Reverting this migration would require restoring arbitrary (likely incorrect)
-- values, which defeats the purpose of the fix.

-- NOOP query required to not break the revertion

SELECT 1;
