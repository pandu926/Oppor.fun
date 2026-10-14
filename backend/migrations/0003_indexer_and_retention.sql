ALTER TABLE indexer_cursors ADD COLUMN observed_safe_head bigint;
ALTER TABLE uploads ADD COLUMN staging_deleted_at timestamptz;
CREATE INDEX uploads_staging_cleanup ON uploads(expires_at) WHERE staging_deleted_at IS NULL;
