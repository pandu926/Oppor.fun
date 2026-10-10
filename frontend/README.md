# Oppor frontend

Vite 8, React 19, TypeScript, React Router, TanStack Query, RainbowKit, wagmi, and viem. The desktop marketplace follows [the approved indigo reference](../design/oppor-indigo-marketplace-v2.png). SVG campaign artwork is rendered in the interface; the mockup is not used as a page background.

## Run locally

Use Node 22.12 or newer.

```sh
cd frontend
npm ci
npm run dev
```

Open `http://127.0.0.1:3000`. The default is **demo mode**, with six sample campaigns. Connect → **Continue in demo mode** enables local participant entries and creator drafts. These are saved in browser storage. No real wallet signature, token allocation, deployment, funding, or claim is simulated as a successful blockchain transaction.

```sh
npm run typecheck
npm run test:unit
npm test
npm run build
npm run preview
```

Tests use Chromium. The configuration uses installed Google Chrome when available; otherwise run `npx playwright install chromium`. `PLAYWRIGHT_CHROMIUM_EXECUTABLE` overrides the executable. Browser tests start a separate live fixture server on port 4311. Both test ports must be available or running this application. Tests never send real blockchain transactions.

## Live backend and wallet configuration

Copy `.env.example` to `.env.local`, set `VITE_DATA_MODE=live`, and restart Vite. Public variables are compiled into the build: **never put secrets or signing keys in them**.

| Variable                        | Purpose                                                                |
| ------------------------------- | ---------------------------------------------------------------------- |
| `VITE_DATA_MODE`                | `demo` by default; `live` enables API and browser wallet operations    |
| `VITE_API_BASE_URL`             | `/v1` for a same-origin reverse proxy; absolute HTTPS URLs also work   |
| `VITE_SITE_URL`                 | Canonical public origin used for SEO                                   |
| `VITE_CHAIN_ID`                 | Must match the backend's configured deployment                         |
| `VITE_RPC_URL`                  | Public HTTPS RPC, with browser CORS                                    |
| `VITE_EXPLORER_URL`             | HTTPS explorer origin                                                  |
| `VITE_FACTORY_ADDRESS`          | Verified deployed factory; required to authorize creation transactions |
| `VITE_USDC_ADDRESS`             | Verified ERC-20 USDC contract for the configured network               |
| `VITE_WALLETCONNECT_PROJECT_ID` | Public Reown project ID; enables WalletConnect QR and mobile wallets   |
| `PRERENDER_API_URL`             | Optional server-only HTTPS public API for campaign HTML snapshots      |

The default is Arc Testnet, chain ID 5042002. RPC and explorer defaults follow [Arc's connection reference](https://docs.arc.io/arc/references/connect-to-arc). Arc's [USDC ERC-20 interface](https://docs.arc.io/integrate/infrastructure/indexing-events) uses six decimals; native gas USDC uses eighteen. Arbitrary ERC-20 metadata is read from the configured RPC and cached; if unavailable, the interface shows base units instead of guessing decimals. The create form accepts exact integer base units, supports all three asset kinds, and validates deadline order, NFT inventory, required tasks, capacity, and fixed-pool arithmetic.

For local development, Vite proxies `/v1` to `http://127.0.0.1:8080`. Configure backend `PUBLIC_ORIGIN` and allowed origins to match the frontend host exactly. `localhost` and `127.0.0.1` are different origins. Configure object-storage POST CORS for that same origin. HTTP RPC, API, explorer, and upload URLs are accepted only on localhost in development; production requires HTTPS.

RainbowKit handles wallet selection and connection; the backend's exact sign-in message separately authorizes an Oppor session. Browser wallets work without a relay account. To enable WalletConnect QR and mobile wallets, set `VITE_WALLETCONNECT_PROJECT_ID` to your public 32-character Reown/WalletConnect project ID and allow your application origins in that project dashboard. Without this setting only the browser-wallet connector is offered; no shared or invented project ID is used. A connected wallet alone does not grant creator or admin access. Transactions use the selected connector's EIP-1193 provider, including WalletConnect, rather than assuming `window.ethereum`. The HttpOnly session cookie is accompanied by an in-memory CSRF token. Reconnect after a page reload to obtain a fresh authorized session. Wallet/account/network changes clear the session UI and cached private data. Admin access is granted only after `/admin/me` authorizes an operator; a role cannot be enabled in browser storage. Expired admin freshness requires reconnecting and signing again.

## Implemented routes

| Route                                 | Behavior                                                                                                                                       |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `/`                                   | Public marketplace, reward tabs, search within loaded campaigns, ending/newest pagination                                                      |
| `/campaigns/:id`                      | Campaign rules, deadlines, entry registration, task evidence, image upload, explicit submission, review status, claims and proof export        |
| `/create`                             | Validated reward, distribution, schedule and task form; saves private draft                                                                    |
| `/campaigns`                          | Creator campaign list, including private drafts                                                                                                |
| `/campaigns/:id/manage`               | Edit draft, lock configuration, deploy, approve/fund, activate, paginated review, eligibility lock, preview/export/finalize, cancel and refund |
| `/entries`                            | Saved and currently discovered participant entries; campaign-ID recovery and more public pages                                                 |
| `/rewards`                            | Allocations, claim actions and proof export for discovered/saved entries                                                                       |
| `/admin`                              | Stats, campaign moderation, users, suspension/restoration, session revocation, audit history, paginated jobs/status filtering                  |
| `/docs`, `/docs/:guide`               | Dedicated documentation workspace with 12 product and developer guides, full-text search, section navigation, and copyable examples            |
| `/setup/deploy`                       | Operator wallet deployment of the release factory on Arc mainnet; fee review, receipt verification, and manifest export                        |
| `/how-it-works`, `/privacy`, `/terms` | Public product explanations and current data-handling information                                                                              |

The current backend does not expose a global participant-entry feed. Entry and reward screens recover locally remembered campaign IDs and check the currently loaded public catalog in batches of four. They disclose this scope and provide ID import and additional catalog pagination. Search and asset tabs operate on the currently loaded catalog; load more to expand results. Token listings do not imply market-price or value guarantees.

## Transaction and request boundaries

Prepared transactions must match the configured network, current wallet, expected factory/escrow/token, requested intent, zero native value, and a valid expiry. Calldata is decoded against the contract ABI: creation must reproduce the locked configuration hash; approvals must name this escrow with bounded amounts or configured NFT IDs; funding must use the configured asset/function; claims must pay the connected wallet; finalization hashes must match the prepared commitments. The wallet independently displays the transaction and fees.

Each action first presents a review dialog. Approval transactions are confirmed one at a time; the UI then prepares funding again, including simulation against updated allowance. Transaction hashes are exposed after broadcast. Receipt confirmation and transaction tracking are not treated as confirmed backend business state; the indexer remains authoritative. A timeout may occur after broadcast: inspect the displayed hash and refresh rather than submitting another transaction blindly.

Mutations send CSRF and idempotency headers; ambiguous network/503/504 results retain the same request key for the same payload in the current session. Version conflicts require refresh/review. Private evidence is escaped as text, links require HTTPS, and external navigation uses `noopener noreferrer`. PNG/JPEG/WebP uploads are limited to five MiB, use a signed PUT URL binding the original file size and content type, and are completed before evidence references the immutable upload. Saving evidence returns an entry to `REGISTERED`; submission is a separate action.

Generated domain types come from the authoritative backend schema:

```sh
python3 scripts/generate-types.py
npm run format
```

Function ABI comes from Foundry artifacts after contract compilation:

```sh
python3 scripts/generate-abi.py
node scripts/generate-deployment-artifact.mjs
npm run format
```

Generated files are committed so frontend builds do not require Rust or Foundry. Review and regenerate them when changing API or contract interfaces.

## Production build and SEO

`npm run build` type-checks, builds browser/SSR bundles, and writes HTML for public informational pages and private authentication gates. Demo pages carry `noindex`, and the demo robots file disallows crawling. Live public pages have canonical metadata, Open Graph/Twitter tags, a PNG social card, JSON-LD, a sitemap, and a robots file. Private workspaces remain `noindex`. Real public campaign details receive their own metadata after loading; use `PRERENDER_API_URL` for crawlable initial HTML of the latest 100 public campaigns. The snapshot build requests public detail only, sends no session credentials, and fails on snapshot errors. Schedule rebuilds if campaign discovery HTML must remain current; runtime requests still refresh authoritative state.

Deploy **`dist/`**, not `dist-server/`. Serve existing `/:route/index.html` first, assets normally, then fall back to `/200.html` for dynamic routes. An `_redirects` fallback is included for hosts that support it. For Nginx:

```nginx
location / {
    try_files $uri $uri/index.html /200.html;
    add_header X-Content-Type-Options nosniff always;
    add_header Referrer-Policy strict-origin-when-cross-origin always;
    add_header X-Frame-Options DENY always;
}
location /assets/ {
    expires 1y;
    add_header Cache-Control "public, immutable";
}
# Configure /v1 reverse proxy separately if API requests are same-origin.
```

Use TLS, short HTML caching, and restrictive API/upload CORS. Set a deployment CSP with `default-src 'self'`, `object-src 'none'`, `base-uri 'self'`, `frame-ancestors 'none'`, local font/style sources, and explicit API/RPC/upload connection origins. The prerendered bootstrap and JSON-LD are inline scripts: authorize their exact build hashes or serve them externally; do not add a blanket `unsafe-inline` script allowance. Private pages and evidence must not be edge-cached.

The live adapter is covered by browser API fixtures; no funded Arc deployment was available for an end-to-end onchain transfer in this frontend task. Deploy/configure the verified factory, backend, RPC and storage before enabling real campaigns. Existing internal contract review is not an independent external audit.

## Product documentation

Documentation content lives in `src/docs/content.ts` and uses structured sections for prose, tables, steps, callouts, and code blocks. Add a page to `docPages` to include it in sidebar navigation, search, next/previous links, metadata, and prerender output. The `/docs` layout is independent of the campaign workspace and remains readable without JavaScript. Search is local and does not send queries to a third-party service. Use Ctrl/⌘ K, arrow keys, Enter, and Escape to navigate search.

Wallet dependencies are pinned to RainbowKit 2.2.11 and wagmi 2.19.5. Transitive overrides select patched `ws`, `uuid`, and `decode-uri-component` versions; retain the build and wallet regression checks when updating them. A production CSP must also allow the configured WalletConnect relay HTTPS/WSS endpoints when that connector is enabled.
