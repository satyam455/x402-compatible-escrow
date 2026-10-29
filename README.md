# AgentPay

AgentPay is a devnet-first reference implementation for programmable AI-agent payments: x402 V2 exact payments, a spending-policy service, signed-verdict boundaries, and a non-custodial Anchor USDC escrow.

## Quick start

```bash
pnpm install
docker compose up -d postgres
pnpm --filter @agentpay/web dev
pnpm --filter @agentpay/x402 dev
cargo run -p agentpay-policy
```

The web app runs on `http://localhost:3000`, x402 on `http://localhost:4021`, and policy on `http://localhost:8787`.

`X402_MODE=mock` is a deliberately labeled challenge simulator and never claims settlement. For real Solana Devnet settlement, set `X402_MODE=real`, configure `X402_PAY_TO`, and verify the facilitator's `/supported` response before starting the resource server.

## Anchor

```bash
solana-test-validator
NO_DNA=1 anchor build
NO_DNA=1 anchor test
```

The escrow program uses SPL Token on Devnet. The default release arbiter is the buyer. A policy verdict authorizes an agent decision offchain; it is not treated as proof of service delivery and cannot release escrow by itself.

## Scope and status

The repository is intentionally explicit about what is real: confirmed onchain state and facilitator receipts are never fabricated. The first milestone includes the production-shaped program, x402 V2 server wiring, policy decision endpoint, dashboard flows, and security documentation. PostgreSQL/indexer persistence and a production wallet-standard adapter are the next integration milestone.
