# Oppor

Token promotion campaigns with creator-funded escrow rewards, manual evidence review, raffle or all-eligible allocation, and recipient-initiated claims.

- [Implemented Solidity factory and escrow](contracts/README.md)
- [Internal contract audit and remaining risks](contracts/AUDIT.md)
- [Implemented Rust backend](backend/README.md)
- [API reference](backend/API.md) and [OpenAPI specification](backend/openapi.json)
- [Platform administration](backend/ADMIN.md)
- [Security boundaries](backend/SECURITY.md)
- [Operations runbook](backend/RUNBOOK.md)
- [Verification results](backend/VERIFICATION.md)
- [Original product and contract specifications](docs/README.md)

The backend uses Axum, SQLx, PostgreSQL, and Redis. It prepares unsigned wallet transactions and indexes verified escrow events. Social evidence is reviewed manually; no X API is used. The factory/escrow implementation supports ERC-20, ERC-721, and ERC-1155 rewards and is exercised by Solidity tests and the backend integration suite. Independent audit and live deployment remain outstanding.
