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
    pip install -r requirements.txt
    python main.py
    ```

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