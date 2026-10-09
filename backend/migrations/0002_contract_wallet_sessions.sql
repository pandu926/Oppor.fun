ALTER TABLE sessions ADD COLUMN contract_wallet boolean NOT NULL DEFAULT false;
ALTER TABLE sessions ADD COLUMN auth_message text;
ALTER TABLE sessions ADD COLUMN auth_signature text;
ALTER TABLE sessions ADD CONSTRAINT contract_session_proof CHECK (
 (NOT contract_wallet AND auth_message IS NULL AND auth_signature IS NULL) OR
 (contract_wallet AND auth_message IS NOT NULL AND length(auth_message)<=4096 AND auth_signature IS NOT NULL AND length(auth_signature)<=8194)
);
