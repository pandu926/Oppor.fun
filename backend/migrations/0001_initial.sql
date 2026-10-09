CREATE DOMAIN evm_address AS bytea CHECK (octet_length(VALUE) = 20);
CREATE DOMAIN evm_hash AS bytea CHECK (octet_length(VALUE) = 32);
CREATE DOMAIN uint256 AS numeric(78,0) CHECK (VALUE >= 0 AND VALUE <= 115792089237316195423570985008687907853269984665640564039457584007913129639935);

CREATE TABLE users (
 id uuid PRIMARY KEY, primary_wallet evm_address NOT NULL UNIQUE,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE auth_nonces (
 nonce_hash evm_hash PRIMARY KEY, wallet evm_address NOT NULL, chain_id bigint NOT NULL CHECK(chain_id>0),
 message_hash evm_hash NOT NULL, expires_at timestamptz NOT NULL, consumed_at timestamptz,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX auth_nonces_expiry ON auth_nonces(expires_at);
CREATE TABLE sessions (
 id uuid PRIMARY KEY, user_id uuid NOT NULL REFERENCES users(id), token_hash evm_hash NOT NULL UNIQUE,
 csrf_hash evm_hash NOT NULL, chain_id bigint NOT NULL CHECK(chain_id>0), expires_at timestamptz NOT NULL,
 revoked_at timestamptz, created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX sessions_user ON sessions(user_id);
CREATE INDEX sessions_expiry ON sessions(expires_at);

CREATE TABLE campaigns (
 id uuid PRIMARY KEY, campaign_key evm_hash NOT NULL, creator_id uuid NOT NULL REFERENCES users(id),
 creator_wallet evm_address NOT NULL, chain_id bigint NOT NULL CHECK(chain_id>0), escrow_address evm_address,
 title text NOT NULL CHECK(length(title) BETWEEN 3 AND 100), description text NOT NULL CHECK(length(description)<=5000),
 status text NOT NULL DEFAULT 'DRAFT' CHECK(status IN ('DRAFT','CONFIG_LOCKED','FUNDING','SCHEDULED','ACTIVE','REVIEWING','ELIGIBILITY_LOCKED','ALLOCATION_READY','CLAIM_OPEN','COMPLETED','CANCELLED','EXPIRED','CLOSED','INTEGRITY_ERROR')),
 mode text NOT NULL CHECK(mode IN ('ALL_ELIGIBLE','RAFFLE')), start_at timestamptz NOT NULL, cutoff_at timestamptz NOT NULL,
 review_deadline timestamptz NOT NULL, claim_deadline timestamptz NOT NULL,
 capacity integer NOT NULL CHECK(capacity BETWEEN 0 AND 100000), registration_limit integer NOT NULL CHECK(registration_limit BETWEEN 1 AND 100000),
 registered_count integer NOT NULL DEFAULT 0 CHECK(registered_count>=0 AND registered_count<=registration_limit),
 winner_count integer NOT NULL CHECK(winner_count BETWEEN 0 AND 100000),
 spec_json jsonb NOT NULL, rules_json jsonb, rules_hash evm_hash, config_hash evm_hash,
 activated boolean NOT NULL DEFAULT false, chain_state smallint NOT NULL DEFAULT 0 CHECK(chain_state BETWEEN 0 AND 4),
 version bigint NOT NULL DEFAULT 0, listed boolean NOT NULL DEFAULT false,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(), updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(chain_id,campaign_key), UNIQUE(chain_id,escrow_address),
 CHECK(start_at<cutoff_at AND cutoff_at<review_deadline AND review_deadline<claim_deadline),
 CHECK((mode='RAFFLE' AND winner_count>0) OR (mode='ALL_ELIGIBLE' AND winner_count=0))
);
CREATE INDEX campaigns_listing ON campaigns(created_at DESC,id DESC) WHERE listed;
CREATE INDEX campaigns_cutoff ON campaigns(cutoff_at) WHERE activated;
CREATE INDEX campaigns_creator ON campaigns(creator_id,created_at DESC,id DESC);

CREATE TABLE campaign_rewards (
 campaign_id uuid PRIMARY KEY REFERENCES campaigns(id), asset_kind smallint NOT NULL CHECK(asset_kind BETWEEN 0 AND 2),
 token_address evm_address NOT NULL, token_id uint256 NOT NULL DEFAULT 0,
 target_amount uint256 NOT NULL CHECK(target_amount>0), funded_amount uint256 NOT NULL DEFAULT 0,
 claimed_amount uint256 NOT NULL DEFAULT 0, swept_amount uint256 NOT NULL DEFAULT 0,
 CHECK(funded_amount<=target_amount AND claimed_amount+swept_amount<=funded_amount)
);
CREATE TABLE tasks (
 id uuid PRIMARY KEY, campaign_id uuid NOT NULL REFERENCES campaigns(id), position integer NOT NULL CHECK(position>=0 AND position<20),
 task_type text NOT NULL CHECK(task_type IN ('X_REPOST','X_LIKE','X_COMMENT','X_TAG','DISCORD_JOIN','CUSTOM')),
 target_url text NOT NULL, instructions text NOT NULL CHECK(length(instructions)<=2000), required boolean NOT NULL,
 UNIQUE(campaign_id,position), UNIQUE(id,campaign_id)
);
CREATE TABLE entries (
 id uuid PRIMARY KEY, campaign_id uuid NOT NULL REFERENCES campaigns(id), user_id uuid NOT NULL REFERENCES users(id),
 payout_wallet evm_address NOT NULL, slot_number integer NOT NULL CHECK(slot_number>0),
 x_username_declared text CHECK(length(x_username_declared)<=64), discord_username_declared text CHECK(length(discord_username_declared)<=100),
 status text NOT NULL DEFAULT 'REGISTERED' CHECK(status IN ('REGISTERED','SUBMITTED','ELIGIBLE','DISQUALIFIED','NOT_SUBMITTED')),
 submitted_at timestamptz, version bigint NOT NULL DEFAULT 0,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(), updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(campaign_id,user_id), UNIQUE(campaign_id,payout_wallet), UNIQUE(campaign_id,slot_number), UNIQUE(id,campaign_id)
);
CREATE INDEX entries_review ON entries(campaign_id,status,id);
CREATE TABLE uploads (
 id uuid PRIMARY KEY, entry_id uuid NOT NULL REFERENCES entries(id), user_id uuid NOT NULL REFERENCES users(id),
 staging_key text NOT NULL UNIQUE, object_key text UNIQUE, content_type text NOT NULL, expected_size bigint NOT NULL CHECK(expected_size BETWEEN 1 AND 5242880),
 content_hash evm_hash, status text NOT NULL DEFAULT 'PENDING' CHECK(status IN ('PENDING','COMPLETE','DELETED')),
 expires_at timestamptz NOT NULL, created_at timestamptz NOT NULL DEFAULT clock_timestamp(), completed_at timestamptz,
 UNIQUE(id,entry_id)
);
CREATE INDEX uploads_cleanup ON uploads(expires_at,status);
CREATE TABLE submissions (
 id uuid PRIMARY KEY, campaign_id uuid NOT NULL REFERENCES campaigns(id), entry_id uuid NOT NULL,
 task_id uuid NOT NULL, revision integer NOT NULL CHECK(revision>0), evidence_json jsonb NOT NULL,
 upload_id uuid, received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 FOREIGN KEY(entry_id,campaign_id) REFERENCES entries(id,campaign_id),
 FOREIGN KEY(task_id,campaign_id) REFERENCES tasks(id,campaign_id),
 FOREIGN KEY(upload_id,entry_id) REFERENCES uploads(id,entry_id), UNIQUE(entry_id,task_id,revision)
);
CREATE INDEX submissions_latest ON submissions(entry_id,task_id,revision DESC);
CREATE TABLE entry_reviews (
 id uuid PRIMARY KEY, entry_id uuid NOT NULL REFERENCES entries(id), reviewer_id uuid NOT NULL REFERENCES users(id),
 decision text NOT NULL CHECK(decision IN ('ELIGIBLE','DISQUALIFIED')), reason text NOT NULL CHECK(length(reason) BETWEEN 1 AND 2000),
 entry_version bigint NOT NULL, created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX reviews_entry ON entry_reviews(entry_id,created_at DESC);

CREATE TABLE eligibility_snapshots (
 id uuid PRIMARY KEY, campaign_id uuid NOT NULL UNIQUE REFERENCES campaigns(id), snapshot_hash evm_hash NOT NULL,
 artifact jsonb NOT NULL, eligible_count integer NOT NULL CHECK(eligible_count>=0 AND eligible_count<=100000),
 locked_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE snapshot_entries (
 snapshot_id uuid NOT NULL REFERENCES eligibility_snapshots(id), ordinal integer NOT NULL CHECK(ordinal>=0),
 entry_id uuid NOT NULL REFERENCES entries(id), payout_wallet evm_address NOT NULL,
 PRIMARY KEY(snapshot_id,ordinal), UNIQUE(snapshot_id,payout_wallet), UNIQUE(snapshot_id,entry_id)
);
CREATE TABLE raffles (
 campaign_id uuid PRIMARY KEY REFERENCES campaigns(id), seed_commitment evm_hash NOT NULL, encrypted_seed bytea NOT NULL,
 revealed_seed evm_hash, algorithm_version text NOT NULL DEFAULT 'raffle-v1', transcript jsonb,
 executed_at timestamptz
);
CREATE TABLE allocation_manifests (
 id uuid PRIMARY KEY, campaign_id uuid NOT NULL UNIQUE REFERENCES campaigns(id), snapshot_id uuid NOT NULL REFERENCES eligibility_snapshots(id),
 root evm_hash NOT NULL, manifest_hash evm_hash NOT NULL, object_key text NOT NULL, artifact jsonb NOT NULL,
 leaf_count integer NOT NULL CHECK(leaf_count BETWEEN 0 AND 100000), allocated_total uint256 NOT NULL,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE allocations (
 id uuid PRIMARY KEY, campaign_id uuid NOT NULL REFERENCES campaigns(id), manifest_id uuid NOT NULL REFERENCES allocation_manifests(id),
 claim_index integer NOT NULL CHECK(claim_index BETWEEN 0 AND 99999), entry_id uuid NOT NULL REFERENCES entries(id),
 recipient evm_address NOT NULL, token_id uint256 NOT NULL, quantity uint256 NOT NULL CHECK(quantity>0),
 leaf_hash evm_hash NOT NULL, proof_json jsonb NOT NULL, claim_tx_hash evm_hash, claimed_at timestamptz,
 UNIQUE(campaign_id,claim_index), UNIQUE(campaign_id,recipient,token_id)
);
CREATE INDEX allocations_wallet ON allocations(recipient,campaign_id);
CREATE TABLE reward_nft_inventory (
 campaign_id uuid NOT NULL REFERENCES campaigns(id), token_id uint256 NOT NULL, deposited boolean NOT NULL DEFAULT false,
 delivered boolean NOT NULL DEFAULT false, allocation_id uuid REFERENCES allocations(id), PRIMARY KEY(campaign_id,token_id)
);
CREATE TABLE chain_transactions (
 chain_id bigint NOT NULL, tx_hash evm_hash NOT NULL, campaign_id uuid NOT NULL REFERENCES campaigns(id),
 kind text NOT NULL CHECK(kind IN ('CREATE','FUND','ACTIVATE','FINALIZE','CLAIM','CANCEL','SWEEP')),
 expected_from evm_address NOT NULL, status text NOT NULL DEFAULT 'PENDING' CHECK(status IN ('PENDING','CONFIRMED','FAILED')),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(), PRIMARY KEY(chain_id,tx_hash)
);
CREATE TABLE chain_events (
 chain_id bigint NOT NULL, block_number bigint NOT NULL, block_hash evm_hash NOT NULL, tx_hash evm_hash NOT NULL,
 log_index integer NOT NULL, address evm_address NOT NULL, event_type text NOT NULL, payload jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(), PRIMARY KEY(chain_id,tx_hash,log_index)
);
CREATE INDEX chain_events_replay ON chain_events(chain_id,block_number,log_index);
CREATE TABLE indexer_cursors (
 chain_id bigint NOT NULL, factory_address evm_address NOT NULL, next_block bigint NOT NULL,
 previous_block_hash evm_hash, updated_at timestamptz NOT NULL DEFAULT clock_timestamp(), halted_reason text,
 PRIMARY KEY(chain_id,factory_address)
);
CREATE TABLE jobs (
 id uuid PRIMARY KEY, kind text NOT NULL CHECK(kind IN ('BUILD_ALLOCATIONS')),
 dedupe_key text NOT NULL UNIQUE, campaign_id uuid NOT NULL REFERENCES campaigns(id),
 state text NOT NULL DEFAULT 'PENDING' CHECK(state IN ('PENDING','RUNNING','SUCCEEDED','FAILED')),
 attempts integer NOT NULL DEFAULT 0, available_at timestamptz NOT NULL DEFAULT clock_timestamp(), locked_until timestamptz,
 worker_id uuid, last_error text, created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX jobs_ready ON jobs(available_at) WHERE state IN ('PENDING','RUNNING');
CREATE TABLE idempotency_keys (
 actor_id uuid NOT NULL REFERENCES users(id), route text NOT NULL, key text NOT NULL CHECK(length(key) BETWEEN 8 AND 128),
 request_hash evm_hash NOT NULL, response_json jsonb, expires_at timestamptz NOT NULL DEFAULT clock_timestamp()+interval '24 hours',
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(), PRIMARY KEY(actor_id,route,key)
);
CREATE TABLE audit_logs (
 id uuid PRIMARY KEY, actor_id uuid REFERENCES users(id), campaign_id uuid REFERENCES campaigns(id),
 action text NOT NULL, metadata jsonb NOT NULL DEFAULT '{}', created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX audit_campaign ON audit_logs(campaign_id,created_at);

-- Defense in depth: locked reward rules cannot be edited through accidental SQL writes.
CREATE FUNCTION protect_campaign_config() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.status <> 'DRAFT' AND (
  NEW.spec_json IS DISTINCT FROM OLD.spec_json OR NEW.creator_id<>OLD.creator_id OR NEW.creator_wallet<>OLD.creator_wallet OR
  NEW.campaign_key<>OLD.campaign_key OR NEW.chain_id<>OLD.chain_id OR NEW.start_at<>OLD.start_at OR NEW.cutoff_at<>OLD.cutoff_at OR
  NEW.review_deadline<>OLD.review_deadline OR NEW.claim_deadline<>OLD.claim_deadline OR NEW.mode<>OLD.mode OR
  NEW.capacity<>OLD.capacity OR NEW.registration_limit<>OLD.registration_limit OR NEW.winner_count<>OLD.winner_count OR
  NEW.rules_json IS DISTINCT FROM OLD.rules_json OR NEW.rules_hash IS DISTINCT FROM OLD.rules_hash OR NEW.config_hash IS DISTINCT FROM OLD.config_hash
 ) THEN RAISE EXCEPTION 'Campaign configuration is locked' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER campaign_config_guard BEFORE UPDATE ON campaigns FOR EACH ROW EXECUTE FUNCTION protect_campaign_config();
CREATE FUNCTION protect_task_config() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE target uuid;
BEGIN
 IF TG_OP='DELETE' THEN target:=OLD.campaign_id; ELSE target:=NEW.campaign_id; END IF;
 IF (SELECT status FROM campaigns WHERE id=target) <> 'DRAFT' THEN RAISE EXCEPTION 'Tasks are locked' USING ERRCODE='23514'; END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; ELSE RETURN NEW; END IF;
END $$;
CREATE TRIGGER task_config_guard BEFORE INSERT OR UPDATE OR DELETE ON tasks FOR EACH ROW EXECUTE FUNCTION protect_task_config();
CREATE FUNCTION append_only() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Immutable record' USING ERRCODE='23514'; END $$;
CREATE TRIGGER audit_immutable BEFORE UPDATE OR DELETE ON audit_logs FOR EACH ROW EXECUTE FUNCTION append_only();
CREATE TRIGGER review_immutable BEFORE UPDATE OR DELETE ON entry_reviews FOR EACH ROW EXECUTE FUNCTION append_only();
CREATE TRIGGER snapshot_immutable BEFORE UPDATE OR DELETE ON eligibility_snapshots FOR EACH ROW EXECUTE FUNCTION append_only();
CREATE TRIGGER snapshot_entries_immutable BEFORE UPDATE OR DELETE ON snapshot_entries FOR EACH ROW EXECUTE FUNCTION append_only();
CREATE TRIGGER manifest_immutable BEFORE UPDATE OR DELETE ON allocation_manifests FOR EACH ROW EXECUTE FUNCTION append_only();
