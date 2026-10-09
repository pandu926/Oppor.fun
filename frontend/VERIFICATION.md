# Frontend verification

Verified locally on 2026-10-09 with Node 22.22.2 and installed Google Chrome.

- Strict TypeScript checking: passed.
- Vite browser build, SSR build, and nine default prerendered routes: passed.
- Prettier formatting check: passed.
- Five unit tests: passed. Covers integer boundaries, URL restrictions, prepared transaction network/sender/target/intent/value/expiry checks, amount formatting, and approval bounds/spender validation.
- Twelve Playwright browser tests: passed. Covers reference geometry and catalog filters, participant registration/evidence/submission/recovery, creator draft/reload, mobile overflow/navigation, keyboard dialog handling, serious/critical accessibility checks, restricted admin/demo access, live API errors without sample fallback, exact-message wallet login/CSRF, creator evidence review, operator moderation version/reason, and rejected claim targets.
- Production preview: six marketplace cards and informational content are present with JavaScript disabled. Demo metadata is `noindex,follow`. Hydration and filter interactions produced no browser errors.
- Desktop capture: 1536 × 1024. Featured panel measured x=302, y=181, width=1205, height=187, within a few pixels of the approved reference.
- Mobile capture: 390 pixels wide; document width is 390, with no horizontal overflow.
- npm installation/audit: zero reported dependency vulnerabilities at installation time.

Screenshots: [desktop](../design/oppor-frontend-desktop.png), [mobile](../design/oppor-frontend-mobile.png). They are captures of the rendered production frontend.

Live HTTP operations were checked with API fixtures, not a funded Arc deployment. No actual token transfer, NFT funding, chain-indexer convergence, live S3 upload, or production operator action was performed. The production backend, storage CORS, verified factory/escrow deployment, and network configuration must be supplied before live use. Public campaign prerender snapshots additionally require `PRERENDER_API_URL`; the default build contains sample data and is deliberately excluded from indexing.
