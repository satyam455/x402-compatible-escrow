# API surface

## x402 resource server

- `GET /api/premium/report` — `$0.01` USDC
- `GET /api/premium/market-data` — `$0.02` USDC
- `GET /api/premium/compute` — `$0.05` USDC
- `GET /health` — mode, network and facilitator configuration

In real mode, the V2 middleware emits `PAYMENT-REQUIRED`, accepts `PAYMENT-SIGNATURE`, and emits `PAYMENT-RESPONSE` after facilitator settlement. Mock mode is visibly labeled and does not claim payment success.

## Policy service

`POST /v1/authorize` accepts a unique request id plus agent, buyer, recipient, amount in base units, mint, CAIP-2 network, category, resource id and expiry. It returns `ALLOW`, `DENY`, or a reason code. The starter service keeps reservations in memory; durable PostgreSQL is required before production deployment.
