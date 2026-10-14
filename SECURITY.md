# Security

Oppor uses creator-funded campaign escrows, wallet authorization, and recipient-bound Merkle claims. The code has internal review and automated tests; it has not undergone an independent security audit.

## Trust boundaries

- Creators review social evidence and determine eligibility. Wallet authentication does not establish a unique human or prove social engagement.
- The creator signs finalization. A Merkle proof establishes inclusion in that finalized allocation, not the fairness of a creator's decision.
- Raffles use a reproducible server draw. The operator controls the committed seed; this is not trustless randomness or VRF.
- Token issuers, restrictive token behavior, and recipient contracts can affect funding, payout, cancellation, and refunds. Review the unresolved NFT refund limitation before funding.
- RPC services, object storage, configured confirmations, and application availability are operational dependencies.

## Technical references

- [Contract security](contracts/SECURITY.md)
- [Internal contract review and remaining risks](contracts/AUDIT.md)
- [Backend security boundaries](backend/SECURITY.md)
- [Verification scope](docs/VERIFICATION.md)

## Reporting a vulnerability

Use this repository's **Security → Report a vulnerability** feature if private vulnerability reporting is enabled. If it is unavailable, open an issue requesting a private contact without including exploit details, credentials, or private evidence. No dedicated security email or bounty program is currently published here.

Do not include private keys, session signatures, cookies, tokens, or participant evidence in public issues or pull requests.

## Credentials and configuration

Keep environment files, signing keys, keystores, backups, service credentials, and deployment-specific infrastructure settings outside version control. Values included in frontend builds are public. Use separate migration and application database identities, restrict metrics and administrative ingress, and follow the [backend operations guide](backend/RUNBOOK.md) for deployment controls.
