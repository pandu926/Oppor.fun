ALTER TABLE chain_transactions ADD COLUMN last_checked_at timestamptz;
CREATE INDEX transaction_reconciliation_queue ON chain_transactions
 (chain_id, (coalesce(last_checked_at,created_at)), created_at) WHERE status='PENDING';
