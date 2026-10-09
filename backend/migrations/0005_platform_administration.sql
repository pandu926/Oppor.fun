ALTER TABLE users ADD COLUMN suspended boolean NOT NULL DEFAULT false;
ALTER TABLE users ADD COLUMN admin_version bigint NOT NULL DEFAULT 0 CHECK(admin_version>=0);
ALTER TABLE users ADD COLUMN suspension_reason text CHECK(length(suspension_reason) BETWEEN 1 AND 2000);
ALTER TABLE users ADD COLUMN suspension_updated_at timestamptz;
ALTER TABLE campaigns ADD COLUMN moderation_hidden boolean NOT NULL DEFAULT false;
ALTER TABLE campaigns ADD COLUMN moderation_version bigint NOT NULL DEFAULT 0 CHECK(moderation_version>=0);
ALTER TABLE campaigns ADD COLUMN moderation_reason text CHECK(length(moderation_reason) BETWEEN 1 AND 2000);
ALTER TABLE campaigns ADD COLUMN moderation_updated_at timestamptz;
CREATE INDEX admin_users_page ON users(created_at DESC,id DESC);
CREATE INDEX admin_campaigns_page ON campaigns(created_at DESC,id DESC);
CREATE INDEX admin_jobs_page ON jobs(created_at DESC,id DESC);
CREATE INDEX admin_audit_page ON audit_logs(created_at DESC,id DESC);
CREATE INDEX admin_failed_jobs ON jobs(created_at DESC,id DESC) WHERE state='FAILED';
CREATE TABLE platform_state (
 singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
 listing_revision bigint NOT NULL DEFAULT 0 CHECK(listing_revision>=0)
);
INSERT INTO platform_state(singleton) VALUES(true);
CREATE FUNCTION advance_listing_revision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.moderation_hidden IS DISTINCT FROM OLD.moderation_hidden THEN
  UPDATE platform_state SET listing_revision=listing_revision+1 WHERE singleton;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER campaign_moderation_cache AFTER UPDATE OF moderation_hidden ON campaigns
 FOR EACH ROW EXECUTE FUNCTION advance_listing_revision();
