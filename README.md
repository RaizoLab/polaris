# 🌟 Polaris: Autonomous DeFi Agent for Stellar

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Network: Stellar](https://img.shields.io/badge/Blockchain-Stellar-blue)](https://stellar.org)
[![Platform: Soroban](https://img.shields.io/badge/Smart%20Contracts-Soroban-purple)](https://soroban.stellar.org)

**Polaris** is an open-source, autonomous financial agent built on the Stellar network. Inspired by infrastructure like Giza's ARMA, Polaris leverages AI to provide non-custodial, 24/7 yield optimization across the Stellar/Soroban DeFi ecosystem.

## 🚀 Overview

Polaris acts as a "Smart Copilot" for your stablecoins. It continuously monitors liquidity pools and lending protocols on Stellar, automatically rebalancing your assets to maximize APY while minimizing risk—all without ever taking custody of your private keys.

### Key Features
* **Non-Custodial:** Your funds stay in your smart account. Polaris only has permission to move assets between whitelisted protocols.
* **Stellar-Native:** Built using **Soroban** smart contracts (Rust) for high speed and low fees.
* **AI-Driven:** Uses the **Stellar AI Agent Kit** to interface with off-chain machine learning models for yield prediction.
* **Verifiable Strategy:** (In-Development) Leveraging ZKML to prove that the agent's decisions follow its open-source strategy.

## 🛠 Tech Stack
* **Smart Contracts:** Rust & Soroban SDK.
* **Agent Logic:** TypeScript via the Stellar AI Agent Kit.
* **DeFi Integration:** Direct adapters for [Blend](https://blend.capital/), [Phoenix](https://phoenix-fi.io/), and [Soroswap](https://soroswap.finance/).
* **Connectivity:** Model Context Protocol (MCP) for seamless AI-to-Contract communication.

## 📂 Repository Structure
* `/contracts`: The core Soroban smart contracts handling vaults and permissions.
* `/agent`: The off-chain engine that analyzes market data and submits transactions.
* `/zkml`: Research and implementation of Zero-Knowledge proofs for model verifiability.

## 🏁 Getting Started

### Prerequisites
* [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) installed.
* Rust and the `wasm32-unknown-unknown` target.
* Python 3.10+ for the agent engine.

### Installation
1.  **Clone the repo:**
    ```bash
    git clone [https://github.com/your-username/polaris.git](https://github.com/your-username/polaris.git)
    cd polaris
    ```
2.  **Build the contracts:**
    ```bash
    cd contracts
    stellar contract build
    ```
3.  **Run the agent locally:**
    ```bash
    cd ../agent
    cp .env.example .env   # set STELLAR_SECRET_KEY (testnet)
    npm install
    pip install -r requirements.txt
    python main.py status
    python main.py hello     # sign + submit Hello World payment
    python main.py yields    # Blend / Phoenix / Soroswap APYs
    python main.py forecast  # 24h ML yield forecast + Optimal Allocation
    python main.py mcp       # start the Polaris MCP server (stdio)
    ```

### MCP Server (LLM access to vault state)

The agent ships an [MCP](https://modelcontextprotocol.io) server so LLM clients (Claude Desktop, Cursor, etc.) can query Polaris directly. Register it with:

```json
{
  "mcpServers": {
    "polaris": {
      "command": "python",
      "args": ["/path/to/polaris/agent/mcp_server.py"]
    }
  }
}
```

Exposed tools:
* `vault_yield` — answers "What is the current yield in my vault?" (blended APY + per-position breakdown)
* `market_scan` — ranks yield opportunities across Blend, Phoenix, and Soroswap
* `rebalance_check` — compares the current allocation to the model-optimal one and recommends hold/rebalance
* `forecast_yields` — 24h ML yield forecast and the Optimal Allocation array

### Transaction Relayer & Dynamic Fee Manager

`agent/src/relayer/` is the execution engine: a background service that listens for Brain signals and submits rebalancing transactions to Soroban.

```bash
cd agent
echo '{"contractId":"C...VAULT","method":"rebalance"}' | npm run relayer
```

Signals arrive as JSON lines on stdin (`{"contractId", "method", "id?"}`); relay results are emitted as JSON lines on stdout. Each signal goes through: fresh sequence number → dynamic fee bid → simulation → resource-fee padding → sign → submit → status polling via Soroban RPC (`getTransaction`). Transient failures (RPC backpressure, timeouts, network errors) retry with exponential backoff and jitter; deterministic failures (simulation errors, on-chain FAILED) abort immediately.

Fees are managed dynamically by `FeeManager`:
* **Inclusion fee** — bids at a live congestion percentile from `getFeeStats` (p70, escalating to p90/p99 on retries), clamped between the network base fee and a hard cap.
* **Resource fee** — every attempt re-simulates against current ledger state and pads the minimum resource fee by 15%, preventing out-of-gas errors during complex rebalances.

### Yield Forecasting Model

`agent/ml/` contains a volatility-aware forecaster: next-24h APY per pool is a mean-reversion blend of the latest rate and the trailing mean, weighted by realized volatility, and `optimal_allocation` converts forecasts into capped, risk-adjusted portfolio weights. Accuracy is verified by a walk-forward backtest against the historical pool-rate dataset in `agent/data/` (`python -m ml.backtest`), which must beat the persistence baseline in CI.

## 🤝 Contributing
Polaris is a community-driven project! We are looking for contributors in the following areas:
* **Rust Developers:** For building and auditing Soroban protocol adapters.
* **Data Scientists:** To refine yield prediction and risk management models.
* **Security Researchers:** To improve the non-custodial permissioning logic.

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for our code of conduct and submission process.

## 📜 License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---
*Built with ❤️ on the Stellar Network.*