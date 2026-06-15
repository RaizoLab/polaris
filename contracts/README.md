# Soroban Contracts

Polaris on-chain smart contracts for vaults, permissions, and SAC interoperability.

## Project Structure

```text
contracts/
├── contracts/
│   ├── auth/           # Non-custodial agent delegation + protocol whitelist
│   ├── vault/          # Core vault: SAC deposits, withdrawals, agent rebalance
│   └── mock-pool/      # Mock liquidity pool for integration testing
├── Cargo.toml
└── README.md
```

## Auth Contract

- `authorize` / `revoke` — User grants or revokes agent delegation (single-tx revoke)
- `add_protocol` / `remove_protocol` — Admin whitelists DeFi protocols
- `require_agent` — Enforces Soroban `require_auth` for the delegated agent

## Vault Contract

- Multi-token SAC `deposit` / `withdraw` with persistent balances
- `agent_deposit_to_pool` — Agent moves liquidity into whitelisted pools
- `rebalance` — Agent moves liquidity between whitelisted pools without user interaction

## Build & Test

```bash
cd contracts/auth && make test
cd ../vault && make test
cd ../mock-pool && make test
```
