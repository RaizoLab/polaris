# Soroban Contracts

Polaris on-chain smart contracts for vaults, permissions, and DeFi protocol adapters.

## Project Structure

```text
contracts/
├── contracts/
│   ├── auth/               # Agent delegation + protocol whitelist
│   ├── vault/              # Core vault with agent operations
│   ├── blend-adapter/      # Blend lending connector (supply / withdraw)
│   ├── phoenix-adapter/    # Phoenix DEX swap connector (slippage protected)
│   ├── mock-blend-pool/    # Mock Blend pool for unit tests
│   ├── mock-phoenix-pool/  # Mock Phoenix pool for unit tests
│   └── mock-pool/          # Generic mock liquidity pool
├── Cargo.toml
└── README.md
```

## Blend Adapter

Abstracts Blend `submit()` requests for vault supply and withdraw:

- `supply(asset, amount)` — `RequestType::Supply` to the configured Blend pool
- `withdraw(asset, amount)` — withdraw supplied assets back to the vault
- Unit tests use `mock-blend-pool` implementing the Blend interface

Vault entry point: `agent_supply_to_blend(blend_adapter, token, amount)`

## Phoenix Adapter

Phoenix-compatible swap with slippage protection:

- `swap(pool, offer_asset, ask_asset, offer_amount, min_ask_amount, max_spread_bps)`
- `quote(pool, offer_asset, offer_amount)` — pre-trade simulation
- Rejects swaps when output is below `min_ask_amount` or spread exceeds `max_spread_bps`

Vault entry point: `agent_swap_phoenix(...)`

## Testnet Verification (Phoenix)

```bash
chmod +x scripts/verify_phoenix_testnet.sh

PHOENIX_POOL_ID=C... \
PHOENIX_ADAPTER_ID=C... \
OFFER_ASSET=C... \
SELL_AMOUNT=1000000 \
SOURCE=your-testnet-account \
  ./scripts/verify_phoenix_testnet.sh
```

## Build & Test

```bash
cd contracts/blend-adapter && make test
cd ../phoenix-adapter && make test
cd ../vault && make test
```
