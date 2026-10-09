# VPS and domain configuration

The application's canonical domain is `https://oppai.fun`. The separate DNS hostname requested for the VPS is `oppee.fun`.

| Setting               | Value                                           |
| --------------------- | ----------------------------------------------- |
| VPS hostname          | `vmi3182679`                                    |
| Public IPv4           | `161.97.103.125`                                |
| Application origin    | `https://oppai.fun`                             |
| Requested DNS records | `oppee.fun`, `www.oppee.fun` → `161.97.103.125` |

## Current DNS status

The supplied Cloudflare account token was verified as active. Its accessible zones include `oppor.fun`, but do not include `oppee.fun` or `oppai.fun`. Public DNS returned NXDOMAIN for `oppee.fun`. `oppai.fun` currently delegates to DigitalOcean nameservers (`ns1.digitalocean.com`, `ns2.digitalocean.com`, `ns3.digitalocean.com`). No records for an alternative spelling were changed.

The requested DNS cannot become active through this Cloudflare account until the exact domain is registered and added as a zone, with its registrar nameservers delegated to Cloudflare. Alternatively, update `oppai.fun` at its current authoritative DNS provider. Registering a domain and changing registrar delegation require that domain owner's access; an API token alone does not establish ownership.

## Apply the prepared records

The credentials supplied for this server are stored outside Git at `/root/.config/oppai/cloudflare.json`, readable only by root. The repository contains no bearer token or R2 secret. The script verifies the exact zone, account, active delegation, and conflicting records before writing. It never edits mail records or records for other names.

```sh
# Read-only plan; fails safely if the requested zone is unavailable.
node ops/sync-dns.mjs
# Apply once the exact zone is active in this Cloudflare account.
node ops/sync-dns.mjs --apply
```

`DNS_ZONE`, `VPS_IPV4`, and `CLOUDFLARE_CREDENTIALS_FILE` override the inputs. The initial records are DNS-only. Enable Cloudflare proxying only after the origin has working TLS and a virtual host for the exact domain.

## Application configuration

For the frontend, set `VITE_SITE_URL=https://oppai.fun` before building. For the backend, set both `PUBLIC_ORIGIN` and `ALLOWED_ORIGINS` to `https://oppai.fun`. Set storage CORS to the same origin and configure trusted reverse proxy addresses explicitly. The application's default canonical URLs and wallet metadata already use this domain.

This VPS already serves other applications on ports 80 and 443 through `cashtokens-nginx`. Add an isolated virtual host to the existing reverse proxy; do not stop or replace that service. Serve `frontend/dist`, preserve the prerendered route files, and proxy `/v1` to the configured Oppor backend. Verified contract deployment addresses, persistent backend services, and origin TLS are still required for real campaign transactions.

RainbowKit browser-wallet connection works without a relay project. WalletConnect QR/mobile connections additionally require `VITE_WALLETCONNECT_PROJECT_ID`, with the application domain allowed in the Reown project dashboard. It is a public project identifier, not a signing secret.
