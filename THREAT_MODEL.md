# Threat model

Assets are buyer USDC, mandate budgets, signed payment payloads, escrow state, and settlement receipts. Threats include replay across orders, wrong mint or token account substitution, unauthorized release, duplicate settlement, expired authorization, RPC notification loss, facilitator failure, and frontend-only authorization bypass.

Onchain mitigations cover PDA derivation, signer checks, mint and token-owner constraints, checked deadline arithmetic, terminal states, and vault closure. Offchain mitigations cover request IDs, expiry, scoped verdicts, rate limits and redacted logs. Remaining work is durable PostgreSQL reservation/reconciliation, signed verdict key rotation, RPC subscription recovery and end-to-end wallet-standard transaction tests.
