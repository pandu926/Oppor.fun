# Frontend verification

The frontend passed strict TypeScript checking, browser and SSR builds, six unit tests, and 32 Playwright regression tests before mainnet activation.

## Coverage

- Campaign browsing, filtering, search, and empty live-data states.
- Creator draft creation, recovery after reload, and cross-platform task selection.
- Participant evidence saving, explicit submission, and recovery.
- Wallet sign-in, exact challenge signing, CSRF handling, account/network changes, and rejected signatures.
- Review decisions, administration authorization, moderation versions, and transaction-target rejection.
- Deadline transitions, refunds, and campaign state display.
- Documentation search, keyboard navigation, anchors, copyable examples, and accessible layouts.
- Mobile navigation and containment at 320, 360, 390, 430, and 768 px, including long titles.

Browser regression tests use explicit development fixtures. Production builds use live API data and never substitute sample campaigns when the API fails.

## Live checks

Mainnet activation was checked in Chrome against the production site, including RainbowKit browser-wallet connection, real SIWE authentication on chain 5042, task configuration, mobile containment, and absence of uncaught JavaScript errors. Those mainnet checks did not broadcast a campaign transaction.

A separate funded testnet browser run exercised campaign creation, deployment, approval, funding, activation, evidence upload, review, finalization, claim, cancellation, and refund. See [project verification](../docs/VERIFICATION.md) for the network and scope of each run.

## Reproduce

```sh
npm run typecheck
npm run format:check
npm run test:unit
npx playwright install chromium
npm test
npm run build
```

See [README.md](README.md) for Node requirements, wallet configuration, browser executables, fixture ports, and prerender behavior.
