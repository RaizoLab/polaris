# Polaris Agent Engine

Off-chain engine for Polaris: Stellar AI Agent Kit integration, Hello World transaction submission, and real-time yield data pipeline.

## Layout

```text
agent/
├── src/                 # TypeScript — Stellar AI Agent Kit + Hello World tx
├── yields/              # Python — Blend / Phoenix / Soroswap APY pipeline
├── main.py              # Python CLI entry (status | hello | yields)
├── .env.example         # Keys + RPC URLs (copy to .env)
├── package.json
└── requirements.txt
```

## Environment

```bash
cp .env.example .env
# Set STELLAR_SECRET_KEY to a funded testnet key (Friendbot).
# Optional: SOROSWAP_API_KEY for live pool APYs.
```

Managed variables:

| Variable | Purpose |
|----------|---------|
| `STELLAR_NETWORK` | `testnet` or `mainnet` |
| `STELLAR_HORIZON_URL` | Horizon REST endpoint |
| `STELLAR_RPC_URL` | Soroban RPC endpoint |
| `STELLAR_SECRET_KEY` | Agent signing key (`S...`) |
| `SOROSWAP_API_KEY` / `BLEND_API_KEY` | Optional protocol API keys |

## Hello World transaction

Signs and submits a native XLM payment with memo `Hello World`:

```bash
npm install
npm run hello
# or
python main.py hello
```

On **mainnet**, `createPolarisAgent()` also initializes [`stellar-agent-kit`](https://www.npmjs.com/package/stellar-agent-kit). Testnet Hello World uses `@stellar/stellar-sdk` against Horizon (kit is mainnet-oriented for DeFi helpers).

## Yield data pipeline

```bash
pip install -r requirements.txt
python main.py yields
# or
python -m yields.fetch_yields --network mainnet
```

Returns standardized JSON:

```json
{
  "network": "mainnet",
  "fetched_at": "2026-07-13T12:00:00+00:00",
  "yields": [
    {
      "protocol": "blend",
      "pool_id": "...",
      "asset": "USDC",
      "apy": 4.2,
      "apr": 4.1,
      "tvl_usd": 1000000,
      "source": "...",
      "status": "ok",
      "error": null
    }
  ],
  "errors": [],
  "summary": { "total": 1, "ok": 1, "failed_protocols": [] }
}
```

Protocol downtime is isolated: a failed Blend/Phoenix/Soroswap call becomes an `error` entry without aborting the whole snapshot.

## Tests

```bash
npm test
pytest
```
