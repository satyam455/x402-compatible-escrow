# Security notes

- No backend or frontend component receives a buyer private key.
- Fund, release, refund and cancel are signer-gated Anchor instructions with PDA, mint, token-owner and status constraints.
- Terminal statuses cannot be changed because every mutating instruction requires `Funded` or `Created` as appropriate.
- Release and refund race safely: the first confirmed transaction changes status and closes the vault; the other fails on status/account state.
- Refund is buyer-only and deadline-gated; it does not require the policy service to be online.
- Policy approval is scoped to agent, buyer, recipient, amount, mint, network, category, resource and expiry. It is not a service-delivery attestation.
- Real x402 mode must validate facilitator support and persist idempotency/receipts before production use. The current local policy store is an in-memory development implementation.
