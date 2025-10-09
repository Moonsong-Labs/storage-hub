DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'service_state'
      AND column_name = 'last_indexed_finalized_block'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'service_state'
      AND column_name = 'last_processed_block'
  ) THEN
    EXECUTE 'ALTER TABLE public.service_state RENAME COLUMN last_indexed_finalized_block TO last_processed_block';
  END IF;
END $$;


