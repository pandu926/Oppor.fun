import { readFile, writeFile, chmod, mkdir } from "node:fs/promises";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
const require = createRequire(
  new URL("../frontend/package.json", import.meta.url),
);
const {
  createPublicClient,
  http,
  defineChain,
  keccak256,
  isAddress,
} = require("viem");
const root = "/root/.config/oppor/production";
const credentials = JSON.parse(
  await readFile(root + "/service-credentials.json", "utf8"),
);
const cloudflare = JSON.parse(
  await readFile("/root/.config/oppor/cloudflare.json", "utf8"),
);
const artifacts = JSON.parse(
  await readFile(
    new URL("../contracts/deployment-artifacts.json", import.meta.url),
    "utf8",
  ),
);
const build = JSON.parse(
  await readFile(
    new URL(
      "../contracts/out/CampaignFactory.sol/CampaignFactory.json",
      import.meta.url,
    ),
    "utf8",
  ),
);
const rpc = "https://rpc.mainnet.arc.io";
const chain = defineChain({
  id: 5042,
  name: "Arc",
  nativeCurrency: { name: "USDC", symbol: "USDC", decimals: 18 },
  rpcUrls: { default: { http: [rpc] } },
});
const client = createPublicClient({
  chain,
  transport: http(rpc, { timeout: 15000 }),
});
if ((await client.getChainId()) !== 5042)
  throw new Error("Arc mainnet RPC chain mismatch.");
let deployment;
let manifestPath = process.argv
  .find((x) => x.startsWith("--manifest="))
  ?.slice(11);
if (!manifestPath) {
  try {
    await readFile(root + "/deployment.verified.json");
    manifestPath = root + "/deployment.verified.json";
  } catch (e) {
    if (e.code !== "ENOENT") throw e;
  }
}
if (manifestPath) {
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  if (
    String(manifest.chain_id) !== "5042" ||
    !isAddress(manifest.factory_address) ||
    !/^0x[0-9a-f]{64}$/i.test(manifest.transaction_hash)
  )
    throw new Error("Invalid Arc mainnet deployment manifest.");
  const receipt = await client.waitForTransactionReceipt({
    hash: manifest.transaction_hash,
    confirmations: 2,
    timeout: 180000,
  });
  if (
    receipt.status !== "success" ||
    receipt.contractAddress?.toLowerCase() !==
      manifest.factory_address.toLowerCase()
  )
    throw new Error("Factory creation receipt does not match the manifest.");
  const transaction = await client.getTransaction({
    hash: manifest.transaction_hash,
  });
  if (
    transaction.to !== null ||
    transaction.input.toLowerCase() !== build.bytecode.object.toLowerCase() ||
    transaction.value !== 0n
  )
    throw new Error(
      "Deployment transaction is not the release factory creation.",
    );
  const code = await client.getBytecode({ address: receipt.contractAddress });
  if (
    !code ||
    keccak256(code) !== artifacts.contracts.CampaignFactory.runtime_code_hash
  )
    throw new Error("Factory runtime bytecode does not match this release.");
  deployment = {
    chain_id: "5042",
    factory_address: receipt.contractAddress,
    deployment_block: receipt.blockNumber.toString(),
    transaction_hash: receipt.transactionHash,
    deployer: receipt.from,
    verified_at: new Date().toISOString(),
    factory_code_hash: artifacts.contracts.CampaignFactory.runtime_code_hash,
    escrow_code_hash: artifacts.contracts.CampaignEscrow.runtime_code_hash,
  };
}
const web = JSON.parse(
  execFileSync("docker", ["inspect", "oppor-web-web-1"], { encoding: "utf8" }),
)[0];
const proxyIp = web.NetworkSettings.Networks["oppor-web_default"]?.IPAddress;
if (!proxyIp || !/^\d+\.\d+\.\d+\.\d+$/.test(proxyIp))
  throw new Error("Cannot identify the exact private frontend proxy address.");
let previousEnvironment = "";
try {
  previousEnvironment = await readFile(root + "/application.env", "utf8");
} catch (e) {
  if (e.code !== "ENOENT") throw e;
}
const previousAdministrators =
  previousEnvironment
    .split("\n")
    .find((line) => line.startsWith("ADMIN_WALLETS="))
    ?.slice(14) || "";
const administrators = (process.env.ADMIN_WALLETS ?? previousAdministrators)
  .split(",")
  .filter(Boolean);
if (administrators.length > 20 || administrators.some((x) => !isAddress(x)))
  throw new Error("Invalid administrator wallet allowlist.");
const env = {
  APP_ENV: "production",
  PUBLIC_ORIGIN: "https://oppor.fun",
  ALLOWED_ORIGINS: "https://oppor.fun",
  DATABASE_MAX_CONNECTIONS: "12",
  DATABASE_URL: `postgres://oppor_app:${credentials.postgres_runtime_password}@postgres:5432/oppor?sslmode=verify-full&sslrootcert=/run/oppor/ca.crt`,
  REDIS_URL: `rediss://:${credentials.redis_password}@redis:6379/0`,
  ARC_CHAIN_ID: "5042",
  ARC_RPC_URL: rpc,
  FACTORY_ADDRESS: deployment?.factory_address || "",
  FACTORY_DEPLOYMENT_BLOCK: deployment?.deployment_block || "",
  FACTORY_CODE_HASH: artifacts.contracts.CampaignFactory.runtime_code_hash,
  ESCROW_CODE_HASH: artifacts.contracts.CampaignEscrow.runtime_code_hash,
  CHAIN_CONFIRMATIONS: "2",
  INDEXER_MAX_LAG_SECONDS: "90",
  SESSION_TTL_SECONDS: "86400",
  RAFFLE_SEED_ENCRYPTION_KEY: credentials.seed_key,
  RATE_LIMIT_KEY: credentials.rate_key,
  OBJECT_STORAGE_ENDPOINT: cloudflare.endpoint,
  OBJECT_STORAGE_BUCKET: "oppor-production",
  OBJECT_STORAGE_REGION: "auto",
  OBJECT_STORAGE_ACCESS_KEY: cloudflare.access_key_id,
  OBJECT_STORAGE_SECRET_KEY: cloudflare.secret_access_key,
  TRUSTED_PROXY_CIDRS: proxyIp + "/32",
  ADMIN_WALLETS: administrators.join(","),
  RUST_LOG: "oppor_backend=info,tower_http=info",
};
async function writeEnv(path, values) {
  await writeFile(
    path,
    Object.entries(values)
      .map(([k, v]) => k + "=" + v)
      .join("\n") + "\n",
    { mode: 0o600 },
  );
  await chmod(path, 0o600);
}
await mkdir(root, { recursive: true, mode: 0o700 });
await writeEnv(root + "/application.env", env);
await writeEnv(root + "/migration.env", {
  APP_ENV: "production",
  DATABASE_URL: `postgres://oppor_migrator:${credentials.postgres_migration_password}@postgres:5432/oppor?sslmode=verify-full&sslrootcert=/run/oppor/ca.crt`,
});
if (deployment) {
  await writeFile(
    root + "/deployment.verified.json",
    JSON.stringify(deployment, null, 2) + "\n",
    { mode: 0o600 },
  );
  await writeEnv(
    new URL("../frontend/.env.production.local", import.meta.url),
    {
      VITE_DATA_MODE: "live",
      VITE_API_BASE_URL: "/v1",
      VITE_SITE_URL: "https://oppor.fun",
      VITE_CHAIN_ID: "5042",
      VITE_RPC_URL: rpc,
      VITE_EXPLORER_URL: "https://explorer.arc.io",
      VITE_FACTORY_ADDRESS: deployment.factory_address,
      VITE_USDC_ADDRESS: "0x3600000000000000000000000000000000000000",
      VITE_WALLETCONNECT_PROJECT_ID:
        process.env.VITE_WALLETCONNECT_PROJECT_ID ??
        (
          await readFile(
            new URL("../frontend/.env.production.local", import.meta.url),
            "utf8",
          ).catch((e) => {
            if (e.code === "ENOENT") return "";
            throw e;
          })
        )
          .split("\n")
          .find((line) => line.startsWith("VITE_WALLETCONNECT_PROJECT_ID="))
          ?.slice(30) ??
        "",
    },
  );
  console.log("Verified Arc mainnet factory:", deployment.factory_address);
} else
  console.log(
    "Infrastructure configuration prepared. Application activation remains gated on a verified factory deployment manifest.",
  );
