# VPS and domain configuration

The application's canonical domain and VPS DNS hostname are `oppor.fun`.

| Setting               | Value                                           |
| --------------------- | ----------------------------------------------- |
| VPS hostname          | `vmi3182679`                                    |
| Public IPv4           | `161.97.103.125`                                |
| Application origin    | `https://oppor.fun`                             |
| Requested DNS records | `oppor.fun`, `www.oppor.fun` → `161.97.103.125` |

## Current DNS status

The supplied Cloudflare account token was verified as active. The account has two active zones: `oppor.fun` and `frostkingdoms.xyz`. The project's domain is `oppor.fun`; earlier spellings in the conversation were typos. The apex `oppor.fun` and `www.oppor.fun` now have DNS-only A records pointing to `161.97.103.125` (TTL 300 seconds). The apex parking CNAME was replaced. The wildcard parking record remains unchanged. Only the project's apex and `www` DNS records are managed by the script.

## Apply the prepared records

The credentials supplied for this server are stored outside Git at `/root/.config/oppor/cloudflare.json`, readable only by root. The repository contains no bearer token or R2 secret. The script verifies the exact zone, account, active delegation, and conflicting records before writing. It never edits mail records or records for other names.

```sh
# Read-only plan; fails safely if the requested zone is unavailable.
node ops/sync-dns.mjs
# Apply once the exact zone is active in this Cloudflare account.
node ops/sync-dns.mjs --apply
```

`DNS_ZONE`, `VPS_IPV4`, and `CLOUDFLARE_CREDENTIALS_FILE` override the inputs. The initial records are DNS-only. Enable Cloudflare proxying only after the origin has working TLS and a virtual host for the exact domain.

## Application configuration

For the frontend, set `VITE_SITE_URL=https://oppor.fun` before building. For the backend, set both `PUBLIC_ORIGIN` and `ALLOWED_ORIGINS` to `https://oppor.fun`. Set storage CORS to the same origin and configure trusted reverse proxy addresses explicitly. The application's default canonical URLs and wallet metadata already use this domain.

This VPS already serves other applications on ports 80 and 443 through `cashtokens-nginx`. Add an isolated virtual host to the existing reverse proxy; do not stop or replace that service. Serve `frontend/dist`, preserve the prerendered route files, and proxy `/v1` to the configured Oppor backend. Verified contract deployment addresses, persistent backend services, and origin TLS are still required for real campaign transactions.

RainbowKit browser-wallet connection works without a relay project. WalletConnect QR/mobile connections additionally require `VITE_WALLETCONNECT_PROJECT_ID`, with the application domain allowed in the Reown project dashboard. It is a public project identifier, not a signing secret.
