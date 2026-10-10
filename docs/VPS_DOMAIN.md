# VPS and domain configuration

The application's canonical domain and VPS DNS hostname are `oppor.fun`.

| Setting               | Value                                           |
| --------------------- | ----------------------------------------------- |
| VPS hostname          | `vmi3182679`                                    |
| Public IPv4           | `161.97.103.125`                                |
| Application origin    | `https://oppor.fun`                             |
| Requested DNS records | `oppor.fun`, `www.oppor.fun` → `161.97.103.125` |

## Current DNS status

The supplied Cloudflare account token was verified as active. The account has two active zones: `oppor.fun` and `frostkingdoms.xyz`. The project's domain is `oppor.fun`; earlier spellings in the conversation were typos. The apex `oppor.fun` and `www.oppor.fun` now have proxied A records pointing to `161.97.103.125` (automatic TTL). The apex parking CNAME was replaced. The wildcard parking record remains unchanged. Only the project's apex and `www` DNS records are managed by the script.

## Apply the prepared records

The credentials supplied for this server are stored outside Git at `/root/.config/oppor/cloudflare.json`, readable only by root. The repository contains no bearer token or R2 secret. The script verifies the exact zone, account, active delegation, and conflicting records before writing. It never edits mail records or records for other names.

```sh
# Read-only plan; fails safely if the requested zone is unavailable.
node ops/sync-dns.mjs
# Apply once the exact zone is active in this Cloudflare account.
node ops/sync-dns.mjs --apply
```

`DNS_ZONE`, `VPS_IPV4`, and `CLOUDFLARE_CREDENTIALS_FILE` override the inputs. Records are proxied by default to use Cloudflare Universal SSL. `DNS_PROXIED=false` is available for diagnostics but disables Cloudflare edge TLS; the origin certificate is trusted by Cloudflare rather than public browsers. Keep proxying enabled for the deployed site.

## Application configuration

For the frontend, set `VITE_SITE_URL=https://oppor.fun` before building. For the backend, set both `PUBLIC_ORIGIN` and `ALLOWED_ORIGINS` to `https://oppor.fun`. Set storage CORS to the same origin and configure trusted reverse proxy addresses explicitly. The application's default canonical URLs and wallet metadata already use this domain.

The frontend is deployed through `ops/compose.web.yml` as `oppor-web-web-1`, with automatic restart and a health check. It serves the production `frontend/dist` build through private port 18820. The shared `cashtokens-nginx` reverse proxy has dedicated virtual hosts from `ops/nginx/origin.conf`, loaded without restarting other applications.

Cloudflare Universal SSL is active for `oppor.fun` and `*.oppor.fun`. The zone uses **Full (strict)** encryption and **Always Use HTTPS**. A Cloudflare Origin CA certificate for `oppor.fun` and `www.oppor.fun` is installed at the reverse proxy; it expires on 10 October 2027. Private key material is stored outside Git and is readable only by root. Renew the origin certificate before that date. HTTP redirects to HTTPS; `www` redirects to the canonical apex.

```sh
docker compose -f ops/compose.web.yml up -d
docker compose -f ops/compose.web.yml ps
# Test public SSL through fresh DNS rather than a cached pre-proxy origin IP.
curl --doh-url https://cloudflare-dns.com/dns-query -I https://oppor.fun/
```

The deployed build currently uses the existing demo data mode. The campaign backend and verified contract deployment are not configured for production. `/v1/` deliberately returns a JSON 503 instead of forwarding to another application's API. Configure the backend, contract deployment, storage, and live frontend environment before enabling real campaign transactions.

RainbowKit browser-wallet connection works without a relay project. WalletConnect QR/mobile connections additionally require `VITE_WALLETCONNECT_PROJECT_ID`, with the application domain allowed in the Reown project dashboard. It is a public project identifier, not a signing secret.
