REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO oppor_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO oppor_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO oppor_app;
REVOKE ALL ON TABLE _sqlx_migrations FROM oppor_app;
REVOKE UPDATE, DELETE ON TABLE audit_logs, entry_reviews, eligibility_snapshots, snapshot_entries, allocation_manifests FROM oppor_app;
