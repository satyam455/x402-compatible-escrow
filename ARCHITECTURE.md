# Architecture

```text
AI agent → x402 resource server → policy service → wallet signature → facilitator → Solana Devnet
                              └→ escrow coordinator/indexer → PostgreSQL → dashboard
```

Direct x402 and escrow settlement are separate paths. `exact` x402 payments are settled by the configured facilitator. Escrow funding and release are Anchor instructions and are never inferred from a successful x402 receipt.

The escrow PDA is derived from `escrow`, buyer, and the immutable 32-byte order id. Its vault is derived from `vault` and the escrow address; the vault authority is a separate PDA. This makes order identity and token custody deterministic while preventing account substitution.
