import { readFile, writeFile, chmod } from "node:fs/promises";
import { randomBytes } from "node:crypto";
import { execFileSync } from "node:child_process";
const path = "/root/.config/oppor/production/service-credentials.json";
const credentials = JSON.parse(await readFile(path, "utf8"));
if (!credentials.postgres_migration_password) {
  credentials.postgres_migration_password = randomBytes(32).toString("hex");
  await writeFile(path, JSON.stringify(credentials), { mode: 0o600 });
  await chmod(path, 0o600);
}
for (const name of ["postgres_migration_password", "postgres_runtime_password"])
  if (!/^[a-f0-9]{64}$/.test(credentials[name]))
    throw new Error("Invalid generated service credential.");
function sql(user, text) {
  try {
    return execFileSync(
      "docker",
      [
        "compose",
        "-f",
        "ops/compose.production.yml",
        "exec",
        "-T",
        "postgres",
        "psql",
        "-U",
        user,
        "-d",
        "oppor",
        "-v",
        "ON_ERROR_STOP=1",
        "-At",
      ],
      { input: text, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] },
    );
  } catch {
    throw new Error(
      "Private database role provisioning failed. Inspect PostgreSQL diagnostics locally.",
    );
  }
}
// The image's bootstrap role remains private and is never used by API/worker.
const migrator = sql(
  "oppor_owner",
  "SELECT count(*) FROM pg_roles WHERE rolname='oppor_migrator';",
).trim();
if (migrator === "0")
  sql(
    "oppor_owner",
    `CREATE ROLE oppor_migrator LOGIN PASSWORD '${credentials.postgres_migration_password}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;`,
  );
const runtime = sql(
  "oppor_owner",
  "SELECT count(*) FROM pg_roles WHERE rolname='oppor_app';",
).trim();
if (runtime === "0")
  sql(
    "oppor_owner",
    `CREATE ROLE oppor_app LOGIN PASSWORD '${credentials.postgres_runtime_password}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;`,
  );
sql(
  "oppor_owner",
  "ALTER DATABASE oppor OWNER TO oppor_migrator; REVOKE CREATE ON SCHEMA public FROM PUBLIC;",
);
console.log(
  "Separated private cluster bootstrap, schema-owner migration, and restricted runtime database identities.",
);
