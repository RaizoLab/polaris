#!/usr/bin/env bash
# Verify Phoenix adapter against a deployed pool on Stellar Testnet.
#
# Prerequisites:
#   - Stellar CLI installed and configured for testnet
#   - PHOENIX_POOL_ID: deployed Phoenix stable pool contract ID
#   - PHOENIX_ADAPTER_ID: deployed Polaris phoenix-adapter contract ID
#   - SOURCE: funded testnet account alias
#
# Usage:
#   PHOENIX_POOL_ID=C... PHOENIX_ADAPTER_ID=C... SOURCE=alice \
#     ./scripts/verify_phoenix_testnet.sh

set -euo pipefail

: "${PHOENIX_POOL_ID:?PHOENIX_POOL_ID is required}"
: "${PHOENIX_ADAPTER_ID:?PHOENIX_ADAPTER_ID is required}"
: "${SOURCE:?SOURCE account alias is required}"

NETWORK="${NETWORK:-testnet}"

echo "Simulating Phoenix pool quote on ${NETWORK}..."
stellar contract invoke \
  --id "${PHOENIX_POOL_ID}" \
  --source "${SOURCE}" \
  --network "${NETWORK}" \
  -- \
  simulate_swap \
  --offer_asset "${OFFER_ASSET:?OFFER_ASSET is required}" \
  --sell_amount "${SELL_AMOUNT:?SELL_AMOUNT is required}"

echo "Phoenix Testnet pool is reachable. Deploy phoenix-adapter and run agent_swap_phoenix from the vault to complete end-to-end verification."
