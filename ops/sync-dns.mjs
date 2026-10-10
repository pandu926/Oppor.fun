import { readFile } from "node:fs/promises";
import { isIP } from "node:net";

// Credentials stay outside the repository; responses never print bearer tokens.
const credentials = JSON.parse(
  await readFile(
    process.env.CLOUDFLARE_CREDENTIALS_FILE ||
      "/root/.config/oppor/cloudflare.json",
    "utf8",
  ),
);
const zoneName = process.env.DNS_ZONE || "oppor.fun";
const address = process.env.VPS_IPV4 || "161.97.103.125";
if (
  !/^(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,}$/i.test(zoneName) ||
  isIP(address) !== 4
)
  throw new Error("Invalid DNS zone or IPv4 address.");
const apply = process.argv.includes("--apply");
const proxied = process.env.DNS_PROXIED !== "false";
async function api(path, method = "GET", body) {
  const response = await fetch("https://api.cloudflare.com/client/v4" + path, {
    method,
    headers: {
      Authorization: "Bearer " + credentials.api_token,
      "Content-Type": "application/json",
    },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(15000),
  });
  const data = await response.json();
  if (!response.ok || !data.success)
    throw new Error(
      "Cloudflare request failed: " + JSON.stringify(data.errors),
    );
  return data.result;
}
const zones = await api("/zones?name=" + encodeURIComponent(zoneName));
const zone = zones.find(
  (z) => z.name === zoneName && z.account.id === credentials.account_id,
);
if (!zone)
  throw new Error(
    `${zoneName} is not an accessible Cloudflare zone. No DNS records were changed.`,
  );
if (zone.status !== "active")
  throw new Error(
    `${zoneName} is not active. Set its registrar nameservers before applying DNS.`,
  );
for (const name of [zoneName, "www." + zoneName]) {
  const records = await api(
    `/zones/${zone.id}/dns_records?name=${encodeURIComponent(name)}`,
  );
  const conflicts = records.filter(
    (r) =>
      r.type === "AAAA" ||
      (r.type === "CNAME" && r.content !== "pixie.porkbun.com"),
  );
  const existing = records.filter(
    (r) =>
      r.type === "A" ||
      (r.type === "CNAME" && r.content === "pixie.porkbun.com"),
  );
  if (conflicts.length || existing.length > 1)
    throw new Error(
      `Conflicting records for ${name}; resolve them before applying.`,
    );
  const payload = {
    type: "A",
    name,
    content: address,
    ttl: proxied ? 1 : 300,
    proxied,
    comment: "Oppor VPS origin",
  };
  if (existing[0]?.content === address && existing[0]?.proxied === proxied) {
    console.log(`${name}: already configured`);
    continue;
  }
  if (apply)
    await api(
      `/zones/${zone.id}/dns_records${existing[0] ? "/" + existing[0].id : ""}`,
      existing[0] ? "PUT" : "POST",
      payload,
    );
  console.log(`${apply ? "Applied" : "Planned"}: A ${name} -> ${address}`);
}
