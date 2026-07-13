/**
 * Optional on-chain Blend yield reader via @blend-capital/blend-sdk.
 *
 * Set BLEND_POOL_IDS=C...,C... and STELLAR_RPC_URL to load supply APYs from ledger state.
 * Falls back gracefully when pools are unset or RPC is unreachable.
 */
import { PoolV2 } from "@blend-capital/blend-sdk";

export interface BlendOnChainYield {
  protocol: "blend";
  pool_id: string;
  asset: string;
  apy: number | null;
  apr: number | null;
  tvl_usd: number | null;
  source: string;
  status: "ok" | "error" | "unavailable";
  error: string | null;
}

function networkConfig() {
  const network = (process.env.STELLAR_NETWORK ?? "mainnet").toLowerCase();
  const rpc =
    process.env.STELLAR_RPC_URL?.trim() ||
    (network === "testnet"
      ? "https://soroban-testnet.stellar.org"
      : "https://mainnet.sorobanrpc.com");
  const passphrase =
    network === "testnet"
      ? "Test SDF Network ; September 2015"
      : "Public Global Stellar Network ; September 2015";
  return { rpc, passphrase, opts: { allowHttp: false } };
}

export async function fetchBlendOnChainYields(
  poolIds: string[] = (process.env.BLEND_POOL_IDS ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean)
): Promise<BlendOnChainYield[]> {
  if (poolIds.length === 0) {
    return [
      {
        protocol: "blend",
        pool_id: "*",
        asset: "*",
        apy: null,
        apr: null,
        tvl_usd: null,
        source: "blend-sdk",
        status: "unavailable",
        error: "BLEND_POOL_IDS not configured",
      },
    ];
  }

  const network = networkConfig();
  const results: BlendOnChainYield[] = [];

  for (const poolId of poolIds) {
    try {
      const pool = await PoolV2.load(network, poolId);
      for (const [assetId, reserve] of pool.reserves.entries()) {
        const supplyApy = Number.isFinite(reserve.estSupplyApy) ? reserve.estSupplyApy * 100 : null;
        const supplyApr = Number.isFinite(reserve.supplyApr) ? reserve.supplyApr * 100 : null;
        results.push({
          protocol: "blend",
          pool_id: poolId,
          asset: assetId,
          apy: supplyApy,
          apr: supplyApr,
          tvl_usd: null,
          source: `blend-sdk:${poolId}`,
          status: supplyApy != null || supplyApr != null ? "ok" : "unavailable",
          error:
            supplyApy != null || supplyApr != null
              ? null
              : "Reserve loaded but APY/APR unavailable",
        });
      }
      if (![...pool.reserves.keys()].length) {
        results.push({
          protocol: "blend",
          pool_id: poolId,
          asset: "*",
          apy: null,
          apr: null,
          tvl_usd: null,
          source: `blend-sdk:${poolId}`,
          status: "unavailable",
          error: "Pool has no reserves",
        });
      }
    } catch (err) {
      results.push({
        protocol: "blend",
        pool_id: poolId,
        asset: "*",
        apy: null,
        apr: null,
        tvl_usd: null,
        source: `blend-sdk:${poolId}`,
        status: "error",
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  return results;
}
