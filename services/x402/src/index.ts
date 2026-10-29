import express from "express";
import { paymentMiddleware, x402ResourceServer } from "@x402/express";
import { ExactSvmScheme } from "@x402/svm/exact/server";
import { HTTPFacilitatorClient } from "@x402/core/server";
import pino from "pino";

const log = pino({ redact: ["req.headers.payment-signature", "req.headers.authorization"] });
const app = express(); const port = Number(process.env.PORT ?? 4021);
const network = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1" as const; const payTo = process.env.X402_PAY_TO; const mode = process.env.X402_MODE ?? "mock";
const resources = {
  "/api/premium/report": { price: "$0.01", description: "Research-grade sample report", body: { report: "Sample report generated from local fixture data", source: "demo-fixture", live: false } },
  "/api/premium/market-data": { price: "$0.02", description: "Delayed sample market snapshot", body: { symbols: [{ symbol: "SOL", price: 148.22, currency: "USD", source: "demo-fixture" }], live: false } },
  "/api/premium/compute": { price: "$0.05", description: "Deterministic compute result", body: { result: "sha256:agentpay-demo", costUnits: 500, live: false } },
} as const;
if (mode === "real" && payTo) {
  const facilitator = new HTTPFacilitatorClient({ url: process.env.X402_FACILITATOR_URL ?? "https://x402.org/facilitator" });
  const server = new x402ResourceServer(facilitator).register(network, new ExactSvmScheme());
  app.use(paymentMiddleware(Object.fromEntries(Object.entries(resources).map(([path, value]) => [`GET ${path}`, { accepts: [{ scheme: "exact", price: value.price, network, payTo }], description: value.description, mimeType: "application/json" }])), server));
} else log.warn({ mode, configured: Boolean(payTo) }, "x402 is in labeled mock challenge mode; no settlement is claimed");
app.get("/health", (_req, res) => res.json({ ok: true, mode, network, facilitator: mode === "real" ? (process.env.X402_FACILITATOR_URL ?? "https://x402.org/facilitator") : null }));
for (const [path, value] of Object.entries(resources)) app.get(path, (req, res) => {
  if (mode === "real") return res.json(value.body);
  if (!req.header("PAYMENT-SIGNATURE")) return res.status(402).set("PAYMENT-REQUIRED", Buffer.from(JSON.stringify({ x402Version: 2, accepts: [{ scheme: "exact", price: value.price, network, payTo: payTo ?? "UNCONFIGURED" }] })).toString("base64")).json({ error: "payment_required", mode: "mock", resource: path });
  return res.set("PAYMENT-RESPONSE", JSON.stringify({ mode: "mock", settled: false, note: "Configure X402_MODE=real for facilitator settlement" })).json(value.body);
});
app.listen(port, () => log.info({ port, mode, network }, "AgentPay x402 resource server listening"));
