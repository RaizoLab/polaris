# Soroban Contracts

Polaris on-chain smart contracts for vaults and permissions.

## Project Structure

```text
contracts/
├── contracts/
│   ├── vault/          # Core vault: user deposits, withdrawals, agent pool routing
│   └── mock-pool/      # Mock liquidity pool for agent integration testing
├── Cargo.toml          # Workspace manifest
└── README.md
```

## Vault Contract

The vault contract provides:

- **`deposit`** — Users deposit tokens; balances stored in `env.storage().persistent()`
- **`withdraw`** — Users withdraw up to their recorded balance
- **`agent_deposit_to_pool`** — Delegated agent moves vault liquidity into the mock pool
- **Events** — `deposit`, `withdraw`, and `agent_dep` events for off-chain indexing

## Build

```bash
cd contracts/vault
make build

cd ../mock-pool
make build
```

## Test

```bash
cd contracts/vault
make test

cd ../mock-pool
make test
```
